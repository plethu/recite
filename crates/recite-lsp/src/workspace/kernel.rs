use std::collections::{BTreeMap, BTreeSet};
use std::fmt::Write as _;

use recite_compiler::{AuthoringKernel, AuthoringRequest, OpenDocument as KernelOpenDocument};
use recite_core::DocumentKey;

use super::project_index::{SavedDocument, SavedProjectIndex};
use super::schema_index::SchemaIndex;
use super::{DiagnosticRefresh, LspWorkspace};
use crate::documents::{OpenDocument, OpenDocumentStore};
use crate::paths::stable_path_identity;
use crate::summary::{FileIdentity, OpenFileScope};

pub(crate) struct KernelPartition {
    pub(super) kernel: AuthoringKernel,
    pub(super) schema: SchemaIndex,
    pub(super) open_owners: BTreeMap<DocumentKey, lsp_types::Uri>,
    pub(super) retired_schema_uris: BTreeSet<String>,
    pub(super) input_fingerprint: super::kernel_rebuild::PartitionInputFingerprint,
}

impl LspWorkspace {
    pub(crate) fn new_kernel(&self) -> AuthoringKernel {
        AuthoringKernel::new()
    }

    pub(crate) fn partition_schemas(&self) -> BTreeMap<String, SchemaIndex> {
        self.partitions
            .iter()
            .map(|(id, partition)| (id.clone(), partition.schema.clone()))
            .collect()
    }

    pub(crate) fn schema_partition_id(&self, uri: &lsp_types::Uri) -> Option<String> {
        self.schema_partition_ids(uri).into_iter().next()
    }

    pub(crate) fn schema_partition_ids(&self, uri: &lsp_types::Uri) -> Vec<String> {
        self.partitions
            .iter()
            .filter(|(_, partition)| partition.schema.matches_uri(uri))
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub(crate) fn partition_id_for_open(&self, document: &OpenDocument) -> Option<String> {
        document
            .identity()
            .saved_path
            .as_deref()
            .and_then(|path| self.saved.partition_for_path(path))
            .or_else(|| Some("standalone".to_owned()))
    }

    pub(crate) fn partition_id_for_saved(&self, document: &SavedDocument) -> Option<String> {
        self.saved
            .partition_for_path(&document.identity.canonical_path)
            .or_else(|| Some("standalone".to_owned()))
    }

    pub(crate) fn partition_id_for_uri(&self, uri: &lsp_types::Uri) -> Option<String> {
        if let Some(document) = self.documents.document(uri) {
            return self.partition_id_for_open(document);
        }
        if let Some(document) = self.saved.document_by_uri(uri) {
            return self.partition_id_for_saved(document);
        }
        self.schema_partition_id(uri)
    }

    pub(crate) fn partition(&self, id: &str) -> Option<&KernelPartition> {
        self.partitions.get(id)
    }

    pub(crate) fn effective_schema_for_partition(&self, id: &str) -> Option<SchemaIndex> {
        let partition = self.partitions.get(id)?;
        if let Some(schema) =
            partition
                .schema
                .overlay_for_documents_in_partition(&self.documents, &self.saved, id)
        {
            return Some(schema);
        }
        if partition
            .schema
            .has_open_match_in_partition(&self.documents, &self.saved, id)
        {
            return None;
        }
        Some(partition.schema.clone())
    }

    pub(crate) fn publish_open_document(&self, document: &OpenDocument) -> DiagnosticRefresh {
        let diagnostics = self
            .partition_id_for_open(document)
            .and_then(|id| self.partitions.get(&id))
            .and_then(|partition| {
                document_key_for_open(document)
                    .filter(|key| partition.open_owners.get(key) == Some(&document.identity().uri))
                    .and_then(|key| partition.kernel.snapshot().document(&key))
            })
            .map_or_else(
                || self.standalone_open_diagnostics(document),
                |doc| doc.diagnostics().to_vec(),
            );
        DiagnosticRefresh::publish_open(document, diagnostics, self.generation)
    }

    pub(crate) fn publish_saved_document(&self, document: &SavedDocument) -> DiagnosticRefresh {
        let diagnostics = self
            .partition_id_for_saved(document)
            .and_then(|id| self.partitions.get(&id))
            .and_then(|partition| {
                document_key_for_saved(document)
                    .and_then(|key| partition.kernel.snapshot().document(&key))
            })
            .map_or_else(Vec::new, |doc| doc.diagnostics().to_vec());
        DiagnosticRefresh::publish_saved(document, diagnostics, self.generation)
    }

    pub(crate) fn effective_open_document_for_partition_key(
        &self,
        partition: &str,
        key: &DocumentKey,
    ) -> Option<&OpenDocument> {
        self.documents.documents().find(|document| {
            self.partition_id_for_open(document).as_deref() == Some(partition)
                && document_key_for_open(document).as_ref() == Some(key)
        })
    }
}

pub(super) fn effective_open_documents<'a>(
    saved: &SavedProjectIndex,
    documents: &'a OpenDocumentStore,
    schema: &SchemaIndex,
    partition: &str,
    retired: BTreeSet<String>,
) -> BTreeMap<DocumentKey, &'a OpenDocument> {
    let mut result = BTreeMap::new();
    for document in documents.documents() {
        let id = document
            .identity()
            .saved_path
            .as_deref()
            .and_then(|path| saved.partition_for_path(path))
            .unwrap_or_else(|| "standalone".to_owned());
        if schema.matches_uri(&document.identity().uri)
            || retired.contains(document.identity().uri.as_str())
        {
            continue;
        }
        if id != partition {
            continue;
        }
        let Some(key) = document_key_for_open(document) else {
            continue;
        };
        result.entry(key).or_insert(document);
    }
    result
}

pub(super) fn authoring_request(
    saved_index: &SavedProjectIndex,
    open_documents: &BTreeMap<DocumentKey, &OpenDocument>,
    partition: &str,
    expected_generation: recite_compiler::SnapshotGeneration,
) -> AuthoringRequest {
    let saved = saved_index
        .documents
        .values()
        .filter(|document| {
            saved_index
                .partition_for_path(&document.identity.canonical_path)
                .as_deref()
                == Some(partition)
        })
        .filter_map(|document| {
            Some(recite_compiler::SavedDocument::new(
                document_key_for_saved(document)?,
                document.text.clone(),
            ))
        })
        .collect::<Vec<_>>();
    let open = open_documents
        .iter()
        .filter(|(_, document)| {
            document
                .identity()
                .saved_path
                .as_deref()
                .and_then(|path| saved_index.partition_for_path(path))
                .unwrap_or_else(|| "standalone".to_owned())
                == partition
        })
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

pub(super) fn standalone_document_key(document: &OpenDocument) -> Option<DocumentKey> {
    document
        .identity()
        .saved_path
        .as_deref()
        .map(stable_path_identity)
        .and_then(|path| document_key(&path))
        .or_else(|| fallback_document_key(document.identity().uri.as_str().as_bytes()))
}
