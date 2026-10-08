use super::{LspWorkspace, SavedDocument};
use lsp_types::Uri;
use recite_core::DocumentKey;
use std::collections::{BTreeMap, BTreeSet};
use std::path::PathBuf;

/// Protocol identities frozen alongside the semantic snapshot. Query workers
/// never re-resolve symlinks or discover new ownership while reading it.
#[derive(Default)]
pub(super) struct QueryIndex {
    partitions: BTreeMap<String, String>,
    schemas: BTreeSet<String>,
    saved: BTreeMap<String, PathBuf>,
    saved_uris: BTreeMap<String, BTreeMap<DocumentKey, Uri>>,
}
impl LspWorkspace {
    pub(super) fn rebuild_query_index(&mut self) {
        let mut index = QueryIndex::default();
        for document in self.saved.documents.values() {
            if let (Some(partition), Some(key)) = (
                self.partition_id_for_saved(document),
                super::document_key_for_saved(document),
            ) {
                index
                    .saved_uris
                    .entry(partition)
                    .or_default()
                    .entry(key)
                    .or_insert_with(|| document.identity.uri.clone());
            }
            for uri in document.query_uris() {
                index.saved.insert(
                    uri.as_str().to_owned(),
                    document.identity.canonical_path.clone(),
                );
                if let Some(partition) = self.partition_id_for_saved(document) {
                    index.partitions.insert(uri.as_str().to_owned(), partition);
                }
            }
        }
        for document in self.documents.documents() {
            let uri = &document.identity().uri;
            if let Some(partition) = self.partition_id_for_open(document) {
                index.partitions.insert(uri.as_str().to_owned(), partition);
            }
            if self.is_schema_document_uri(uri) {
                index.schemas.insert(uri.as_str().to_owned());
            }
        }
        for partition in self.partitions.values() {
            if let Some(uri) = partition.schema.protocol_uri() {
                if let Some(id) = self.partition_id_for_uri(&uri) {
                    index
                        .partitions
                        .entry(uri.as_str().to_owned())
                        .or_insert(id);
                }
                index.schemas.insert(uri.as_str().to_owned());
            }
        }
        self.query_index = std::sync::Arc::new(index);
    }
    pub(super) fn query_partition_for_uri(&self, uri: &lsp_types::Uri) -> Option<String> {
        self.query_index.partitions.get(uri.as_str()).cloned()
    }
    pub(super) fn query_is_schema(&self, uri: &lsp_types::Uri) -> bool {
        self.query_index.schemas.contains(uri.as_str())
    }
    pub(super) fn query_saved_document(&self, uri: &lsp_types::Uri) -> Option<&SavedDocument> {
        self.saved
            .documents
            .get(self.query_index.saved.get(uri.as_str())?)
    }
    pub(super) fn query_saved_uri(&self, partition: &str, key: &DocumentKey) -> Option<&Uri> {
        self.query_index.saved_uris.get(partition)?.get(key)
    }
    /// Cached ownership only: the coordinator never resolves paths or parses schemas.
    pub(crate) fn query_scopes(&self) -> BTreeMap<String, String> {
        self.query_index
            .partitions
            .iter()
            .filter(|(uri, _)| !self.query_index.schemas.contains(*uri))
            .map(|(uri, partition)| (uri.clone(), partition.clone()))
            .collect()
    }
}
