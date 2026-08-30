use std::collections::BTreeMap;
use std::fmt::Write as _;

use recite_compiler::{AuthoringKernel, AuthoringRequest, OpenDocument as KernelOpenDocument};
use recite_core::DocumentKey;

use super::project_index::SavedDocument;
use super::{DiagnosticRefresh, LspWorkspace};
use crate::documents::OpenDocument;
use crate::summary::FileIdentity;

impl LspWorkspace {
    pub(crate) fn rebuild_next_generation(&mut self) {
        self.generation = super::SnapshotGeneration(self.generation.0.saturating_add(1));
        self.rebuild_kernel();
        self.snapshot = super::LiveProjectSnapshot::rebuild(
            self.generation,
            &self.saved,
            &self.documents,
            self.kernel.snapshot(),
        );
    }

    pub(crate) fn rebuild_kernel(&mut self) {
        let request = self.authoring_request();
        let expected = self.kernel.snapshot().generation();
        if let Err(error) = self.kernel.apply(request) {
            panic!("LSP authoring request invariant violated at generation {expected}: {error}");
        }
    }

    pub(crate) fn new_kernel(&self) -> AuthoringKernel {
        self.schema
            .schema()
            .cloned()
            .map_or_else(AuthoringKernel::new, AuthoringKernel::with_schema)
    }

    fn authoring_request(&self) -> AuthoringRequest {
        let saved = self
            .saved
            .documents
            .values()
            .filter_map(|document| {
                Some(recite_compiler::SavedDocument::new(
                    document_key_for_saved(document)?,
                    document.text.clone(),
                ))
            })
            .collect::<Vec<_>>();
        let mut open_by_key = BTreeMap::new();
        for document in self.documents.documents() {
            let Some(key) = document_key_for_open(document) else {
                continue;
            };
            // URI order is deterministic; the first URI owns a canonical
            // document key when multiple aliases are open concurrently.
            open_by_key.entry(key).or_insert(document);
        }
        let open = open_by_key
            .into_iter()
            .map(|(key, document)| {
                KernelOpenDocument::new(
                    key,
                    recite_compiler::DocumentVersion::new(i64::from(document.version())),
                    document.text().to_owned(),
                )
            })
            .collect::<Vec<_>>();
        AuthoringRequest::new(self.kernel.snapshot().generation(), saved, open)
    }

    pub(crate) fn publish_open_document(&self, document: &OpenDocument) -> DiagnosticRefresh {
        let diagnostics = document_key_for_open(document)
            .filter(|key| self.is_effective_open(document, key))
            .and_then(|key| self.kernel.snapshot().document(&key))
            .map_or_else(Vec::new, |document| document.diagnostics().to_vec());
        DiagnosticRefresh::publish_open(document, diagnostics, self.generation)
    }

    pub(crate) fn publish_saved_document(&self, document: &SavedDocument) -> DiagnosticRefresh {
        let diagnostics = document_key_for_saved(document)
            .and_then(|key| self.kernel.snapshot().document(&key))
            .map_or_else(Vec::new, |document| document.diagnostics().to_vec());
        DiagnosticRefresh::publish_saved(document, diagnostics, self.generation)
    }

    fn is_effective_open(&self, document: &OpenDocument, key: &DocumentKey) -> bool {
        self.documents
            .documents()
            .find(|candidate| document_key_for_open(candidate).as_ref() == Some(key))
            .is_some_and(|candidate| candidate.identity().uri == document.identity().uri)
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
            .or_else(|| fallback_document_key(identity.canonical_path.to_string_lossy().as_ref())),
        FileIdentity::Open(identity) => identity
            .project_relative_path
            .as_deref()
            .and_then(document_key)
            .or_else(|| {
                identity
                    .saved_path
                    .as_deref()
                    .map(|path| path.to_string_lossy())
                    .and_then(|path| fallback_document_key(path.as_ref()))
            })
            .or_else(|| fallback_document_key(identity.uri.as_str())),
    }
}

fn document_key(value: &str) -> Option<DocumentKey> {
    DocumentKey::new(value.to_owned()).ok()
}

fn fallback_document_key(value: &str) -> Option<DocumentKey> {
    let mut encoded = String::from("~lsp/");
    for byte in value.as_bytes() {
        let _ = write!(&mut encoded, "{byte:02x}");
    }
    document_key(&encoded)
}
