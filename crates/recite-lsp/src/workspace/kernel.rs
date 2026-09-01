use std::collections::{BTreeMap, BTreeSet};

use recite_compiler::{AuthoringKernel, AuthoringRequest, OpenDocument as KernelOpenDocument};
use recite_core::DocumentKey;

use super::project_index::{SavedDocument, SavedProjectIndex};
use super::schema_index::SchemaIndex;
use super::{DiagnosticRefresh, LspWorkspace};
use super::{document_key_for_open, document_key_for_saved};
use crate::documents::{OpenDocument, OpenDocumentStore};

#[cfg(test)]
mod tests;

pub(crate) struct KernelPartition {
    pub(super) kernel: AuthoringKernel,
    pub(super) build_id: u64,
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

    pub(crate) fn partition_id_for_open(&self, document: &OpenDocument) -> Option<String> {
        document
            .identity()
            .saved_path
            .as_deref()
            .and_then(|path| self.saved.partition_for_open_path(path))
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
    // Retirement belongs to a schema target, not only to the URI that first
    // opened it. Keep every alias of a still-retired target out of authoring,
    // including an alias reopened after its first close.
    let retired_targets = documents
        .documents()
        .filter(|document| retired.contains(document.identity().uri.as_str()))
        .filter_map(|document| document.identity().saved_path.as_deref())
        .map(crate::paths::stable_path_identity)
        .collect::<BTreeSet<_>>();
    for document in documents.documents() {
        let id = document
            .identity()
            .saved_path
            .as_deref()
            .and_then(|path| saved.partition_for_open_path(path))
            .unwrap_or_else(|| "standalone".to_owned());
        let retired_target = document
            .identity()
            .saved_path
            .as_deref()
            .map(crate::paths::stable_path_identity)
            .is_some_and(|target| retired_targets.contains(&target));
        if schema.matches_uri(&document.identity().uri)
            || retired.contains(document.identity().uri.as_str())
            || retired_target
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
                .and_then(|path| saved_index.partition_for_open_path(path))
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
