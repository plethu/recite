use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use recite_compiler::{AuthoringKernel, AuthoringRequest, OpenDocument as KernelOpenDocument};
use recite_core::DocumentKey;

use super::project_index::{SavedDocument, SavedProjectIndex};
use super::schema_index::SchemaIndex;
use super::{DiagnosticRefresh, LspWorkspace};
use crate::documents::OpenDocument;
use crate::documents::OpenDocumentStore;
use crate::paths::stable_path_identity;
use crate::summary::{FileIdentity, OpenFileScope};

impl LspWorkspace {
    pub(crate) fn rebuild_kernel(&mut self) -> Result<(), recite_compiler::AuthoringError> {
        let saved = self.saved.clone();
        let documents = self.documents.clone();
        let retired_schema_uris = self.retired_schema_uris.clone();
        let owners = self.rebuild_kernel_for(&saved, &documents, None, &retired_schema_uris)?;
        self.kernel_open_owners = owners;
        Ok(())
    }

    pub(super) fn rebuild_state_with_schema(
        &mut self,
        saved: SavedProjectIndex,
        documents: OpenDocumentStore,
        schema: SchemaIndex,
        retired_schema_uris: BTreeSet<String>,
    ) -> Result<(), recite_compiler::AuthoringError> {
        let owners =
            self.rebuild_kernel_for(&saved, &documents, Some(&schema), &retired_schema_uris)?;
        let generation = self.next_generation();
        let snapshot = super::LiveProjectSnapshot::rebuild(
            generation,
            &saved,
            &documents,
            self.kernel.snapshot(),
        );
        self.saved = saved;
        self.documents = documents;
        self.kernel_open_owners = owners;
        self.schema = schema;
        self.retired_schema_uris = retired_schema_uris;
        self.generation = generation;
        self.snapshot = snapshot;
        Ok(())
    }

    fn rebuild_kernel_for(
        &mut self,
        saved: &SavedProjectIndex,
        documents: &OpenDocumentStore,
        schema: Option<&SchemaIndex>,
        retired_schema_uris: &BTreeSet<String>,
    ) -> Result<BTreeMap<DocumentKey, lsp_types::Uri>, recite_compiler::AuthoringError> {
        let schema_index = schema.unwrap_or(&self.schema);
        let open_documents = Self::effective_open_documents(documents)
            .into_iter()
            .filter(|(_, document)| {
                !schema_index.matches_uri(&document.identity().uri)
                    && !retired_schema_uris.contains(document.identity().uri.as_str())
            })
            .collect::<BTreeMap<_, _>>();
        let owners = open_documents
            .iter()
            .map(|(key, document)| (key.clone(), document.identity().uri.clone()))
            .collect::<BTreeMap<_, _>>();
        if schema.is_some() || owners != self.kernel_open_owners {
            let mut kernel = schema.map_or_else(|| self.new_kernel(), Self::new_kernel_for_schema);
            let expected = kernel.snapshot().generation();
            let request = self.authoring_request(saved, &open_documents, expected);
            kernel.apply(request)?;
            self.kernel = kernel;
        } else {
            let expected = self.kernel.snapshot().generation();
            let request = self.authoring_request(saved, &open_documents, expected);
            self.kernel.apply(request)?;
        }
        Ok(owners)
    }

    fn next_generation(&self) -> super::SnapshotGeneration {
        super::SnapshotGeneration(self.generation.0.saturating_add(1))
    }

    pub(crate) fn new_kernel(&self) -> AuthoringKernel {
        Self::new_kernel_for_schema(&self.schema)
    }

    fn new_kernel_for_schema(schema: &SchemaIndex) -> AuthoringKernel {
        schema
            .schema()
            .cloned()
            .map_or_else(AuthoringKernel::new, AuthoringKernel::with_schema)
    }

    fn authoring_request(
        &self,
        saved_index: &SavedProjectIndex,
        open_documents: &BTreeMap<DocumentKey, &OpenDocument>,
        expected_generation: recite_compiler::SnapshotGeneration,
    ) -> AuthoringRequest {
        let saved = saved_index
            .documents
            .values()
            .filter_map(|document| {
                Some(recite_compiler::SavedDocument::new(
                    document_key_for_saved(document)?,
                    document.text.clone(),
                ))
            })
            .collect::<Vec<_>>();
        let open = open_documents
            .iter()
            .map(|(key, document)| {
                KernelOpenDocument::new(
                    key.clone(),
                    recite_compiler::DocumentVersion::new(i64::from(document.version())),
                    document.text().to_owned(),
                )
            })
            .collect::<Vec<_>>();
        AuthoringRequest::new(expected_generation, saved, open)
    }

    fn effective_open_documents(
        documents: &OpenDocumentStore,
    ) -> BTreeMap<DocumentKey, &OpenDocument> {
        let mut open_by_key = BTreeMap::new();
        for document in documents.documents() {
            let Some(key) = document_key_for_open(document) else {
                continue;
            };
            // URI order is deterministic; the first URI owns a canonical
            // document key when multiple aliases are open concurrently.
            open_by_key.entry(key).or_insert(document);
        }
        open_by_key
    }

    pub(crate) fn publish_open_document(&self, document: &OpenDocument) -> DiagnosticRefresh {
        let diagnostics = document_key_for_open(document)
            .filter(|key| self.is_effective_open(document, key))
            .and_then(|key| self.kernel.snapshot().document(&key))
            .map_or_else(
                || self.standalone_open_diagnostics(document),
                |document| document.diagnostics().to_vec(),
            );
        DiagnosticRefresh::publish_open(document, diagnostics, self.generation)
    }

    pub(crate) fn publish_saved_document(&self, document: &SavedDocument) -> DiagnosticRefresh {
        let diagnostics = document_key_for_saved(document)
            .and_then(|key| self.kernel.snapshot().document(&key))
            .map_or_else(Vec::new, |document| document.diagnostics().to_vec());
        DiagnosticRefresh::publish_saved(document, diagnostics, self.generation)
    }

    fn is_effective_open(&self, document: &OpenDocument, key: &DocumentKey) -> bool {
        self.effective_open_document_for_key(key)
            .is_some_and(|candidate| candidate.identity().uri == document.identity().uri)
    }

    pub(crate) fn effective_open_document_for_key(
        &self,
        key: &DocumentKey,
    ) -> Option<&OpenDocument> {
        self.documents
            .documents()
            .find(|document| document_key_for_open(document).as_ref() == Some(key))
    }
}

pub(crate) fn document_key_for_saved(document: &SavedDocument) -> Option<DocumentKey> {
    document_key_for_identity(&FileIdentity::Saved(document.identity.clone()))
}

pub(crate) fn document_key_for_open(document: &OpenDocument) -> Option<DocumentKey> {
    document_key_for_identity(&FileIdentity::Open(document.identity().clone()))
}

pub(crate) fn document_key_for_identity(identity: &FileIdentity) -> Option<DocumentKey> {
    match identity {
        FileIdentity::Saved(identity) => document_key(identity.project_relative_path.as_str())
            .or_else(|| document_key(&stable_path_identity(&identity.canonical_path))),
        FileIdentity::Open(identity) if identity.scope == OpenFileScope::Excluded => None,
        FileIdentity::Open(identity) => identity
            .project_relative_path
            .as_deref()
            .and_then(document_key)
            .or_else(|| {
                identity
                    .saved_path
                    .as_deref()
                    .map(stable_path_identity)
                    .and_then(|path| document_key(&path))
            })
            .or_else(|| fallback_document_key(identity.uri.as_str().as_bytes())),
    }
}

fn document_key(value: &str) -> Option<DocumentKey> {
    DocumentKey::new(value.to_owned()).ok()
}

fn fallback_document_key(value: &[u8]) -> Option<DocumentKey> {
    let mut encoded = String::from("~lsp/");
    for byte in value {
        let _ = write!(&mut encoded, "{byte:02x}");
    }
    document_key(&encoded)
}

impl LspWorkspace {
    fn standalone_open_diagnostics(&self, document: &OpenDocument) -> Vec<recite_core::Diagnostic> {
        if self.is_schema_document_uri(&document.identity().uri) {
            return Vec::new();
        }
        let Some(key) = standalone_document_key(document) else {
            return Vec::new();
        };
        let mut kernel = self.new_kernel();
        let request = AuthoringRequest::new(
            kernel.snapshot().generation(),
            Vec::new(),
            vec![KernelOpenDocument::new(
                key.clone(),
                recite_compiler::DocumentVersion::new(i64::from(document.version())),
                document.text().to_owned(),
            )],
        );
        kernel
            .apply(request)
            .ok()
            .and_then(|_| kernel.snapshot().document(&key))
            .map_or_else(Vec::new, |document| document.diagnostics().to_vec())
    }
}

fn standalone_document_key(document: &OpenDocument) -> Option<DocumentKey> {
    document
        .identity()
        .saved_path
        .as_deref()
        .map(stable_path_identity)
        .and_then(|path| document_key(&path))
        .or_else(|| fallback_document_key(document.identity().uri.as_str().as_bytes()))
}
