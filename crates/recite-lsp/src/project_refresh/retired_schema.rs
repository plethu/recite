use std::collections::{BTreeMap, BTreeSet};

use super::super::kernel::KernelPartition;
use super::super::schema_index::SchemaIndex;
use crate::documents::OpenDocumentStore;

pub(super) fn update_retired_schema_state(
    old_partitions: &BTreeMap<String, KernelPartition>,
    old_documents: &OpenDocumentStore,
    schemas: &BTreeMap<String, SchemaIndex>,
    mut retired_partitions: BTreeMap<String, BTreeSet<String>>,
    mut retired_workspace: BTreeSet<String>,
) -> (BTreeMap<String, BTreeSet<String>>, BTreeSet<String>) {
    for old in old_partitions.values() {
        for document in old_documents.documents() {
            if old.schema.matches_uri(&document.identity().uri) {
                retired_workspace.insert(document.identity().uri.as_str().to_owned());
            }
        }
    }
    retired_workspace.retain(|uri| !matches_any_schema(uri, schemas));
    for uris in retired_partitions.values_mut() {
        uris.retain(|uri| !matches_any_schema(uri, schemas));
    }
    (retired_partitions, retired_workspace)
}

fn matches_any_schema(uri: &str, schemas: &BTreeMap<String, SchemaIndex>) -> bool {
    schemas.values().any(|schema| {
        uri.parse::<lsp_types::Uri>()
            .ok()
            .is_some_and(|uri| schema.matches_uri(&uri))
    })
}
