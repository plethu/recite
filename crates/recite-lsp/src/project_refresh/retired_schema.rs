use std::collections::{BTreeMap, BTreeSet};

use super::super::kernel::KernelPartition;
use super::super::schema_index::SchemaIndex;
use crate::documents::OpenDocumentStore;
use crate::paths::stable_path_identity;

type RetiredSchemaState = (
    BTreeMap<String, BTreeSet<String>>,
    BTreeSet<String>,
    BTreeMap<String, String>,
);

pub(super) fn update_retired_schema_state(
    old_partitions: &BTreeMap<String, KernelPartition>,
    old_documents: &OpenDocumentStore,
    schemas: &BTreeMap<String, SchemaIndex>,
    mut retired_partitions: BTreeMap<String, BTreeSet<String>>,
    mut retired_workspace: BTreeSet<String>,
    mut retired_targets: BTreeMap<String, String>,
) -> RetiredSchemaState {
    for old in old_partitions.values() {
        let target = old
            .schema
            .configured_path()
            .and_then(|path| std::fs::canonicalize(path).ok())
            .map(|path| stable_path_identity(&path));
        for document in old_documents.documents() {
            if old.schema.matches_uri(&document.identity().uri) {
                let uri = document.identity().uri.as_str().to_owned();
                retired_workspace.insert(uri.clone());
                if let Some(target) = target.clone() {
                    retired_targets.insert(uri, target);
                }
            }
        }
    }
    retired_workspace.retain(|uri| !matches_any_schema(uri, schemas));
    for uris in retired_partitions.values_mut() {
        uris.retain(|uri| !matches_any_schema(uri, schemas));
    }
    retired_targets.retain(|_, target| {
        !schemas
            .values()
            .any(|schema| schema.target_identity().as_deref() == Some(target.as_str()))
    });
    (retired_partitions, retired_workspace, retired_targets)
}

fn matches_any_schema(uri: &str, schemas: &BTreeMap<String, SchemaIndex>) -> bool {
    schemas.values().any(|schema| {
        uri.parse::<lsp_types::Uri>()
            .ok()
            .is_some_and(|uri| schema.matches_uri(&uri))
    })
}
