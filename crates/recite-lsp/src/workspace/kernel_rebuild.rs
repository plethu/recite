use std::collections::{BTreeMap, BTreeSet};
use std::sync::Arc;

use recite_compiler::authoring::{AuthoringError, AuthoringKernel};

use super::kernel::{KernelPartition, effective_open_documents};
use super::project_index::SavedProjectIndex;
use super::schema_index::SchemaIndex;
use super::{LspWorkspace, SnapshotGeneration};
use crate::documents::OpenDocumentStore;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct PartitionInputFingerprint {
    saved: Vec<(String, InputText)>,
    open: Vec<(String, String, i32, InputText)>,
    schema: SchemaIndex,
    retired: BTreeSet<String>,
    retired_targets: BTreeSet<String>,
    project_complete: bool,
}

/// Immutable text identity is a sufficient equality proof. `Arc<str>` itself
/// may still scan bytes on the supported toolchain, so make this explicit.
#[derive(Clone, Debug, Eq)]
struct InputText(Arc<str>);

impl PartialEq for InputText {
    fn eq(&self, other: &Self) -> bool {
        Arc::ptr_eq(&self.0, &other.0) || self.0 == other.0
    }
}

impl LspWorkspace {
    pub(super) fn rebuild_for_documents_with_schemas(
        &mut self,
        saved: SavedProjectIndex,
        documents: OpenDocumentStore,
        schemas: BTreeMap<String, SchemaIndex>,
    ) -> Result<(), AuthoringError> {
        let retired = self
            .partitions
            .iter()
            .map(|(id, partition)| (id.clone(), partition.retired_schema_uris.clone()))
            .collect();
        self.rebuild_for_documents_with_schemas_and_retired(saved, documents, schemas, retired)
    }

    pub(super) fn rebuild_for_documents_with_schemas_and_retired(
        &mut self,
        saved: SavedProjectIndex,
        documents: OpenDocumentStore,
        schemas: BTreeMap<String, SchemaIndex>,
        retired: BTreeMap<String, BTreeSet<String>>,
    ) -> Result<(), AuthoringError> {
        // Keep the committed state available until the complete candidate and
        // its final cancellation checkpoint succeed. Failure needs no rollback.
        let generation = SnapshotGeneration(self.generation.0.checked_add(1).ok_or(
            AuthoringError::GenerationExhausted {
                current: recite_compiler::authoring::SnapshotGeneration::new(self.generation.0),
            },
        )?);
        let mut next_partition_build_id = self.next_partition_build_id;
        let mut ids = saved.partition_ids();
        ids.extend(schemas.keys().cloned());
        // A newly opened lexical alias may resolve to a target that was
        // retired before this URI existed in the open-document store.  Carry
        // that exact URI into the candidate state so close/owner accounting
        // remains target-aware after this rebuild commits.
        let mut retired_schema_targets = self.retired_schema_targets.clone();
        let known_retired_targets = retired_schema_targets
            .values()
            .cloned()
            .collect::<BTreeSet<_>>();
        for document in documents.documents() {
            if schemas
                .values()
                .any(|schema| schema.matches_uri(&document.identity().uri))
            {
                continue;
            }
            let Some(current_target) = document
                .identity()
                .saved_path
                .as_deref()
                .map(crate::paths::stable_path_identity)
            else {
                continue;
            };
            if let Some(target) = known_retired_targets
                .iter()
                .find(|target| **target == current_target)
            {
                retired_schema_targets
                    .entry(document.identity().uri.as_str().to_owned())
                    .or_insert_with(|| target.clone());
            }
        }
        let mut retired_all = retired
            .values()
            .flat_map(|uris| uris.iter().cloned())
            .chain(self.retired_schema_uris.iter().cloned())
            .chain(retired_schema_targets.keys().cloned())
            .collect::<BTreeSet<_>>();
        let retired_targets = retired_schema_targets
            .values()
            .cloned()
            .collect::<BTreeSet<_>>();
        // A schema target is a document-level exclusion for the whole
        // workspace.  A shared target may be configured by one partition but
        // must never be parsed as dialogue by a sibling partition.
        for document in documents.documents() {
            if schemas
                .values()
                .any(|schema| schema.matches_uri(&document.identity().uri))
            {
                retired_all.insert(document.identity().uri.as_str().to_owned());
            }
        }
        let mut partitions = BTreeMap::new();
        for id in ids {
            self.control.checkpoint()?;
            let base_schema = schemas.get(&id).cloned().unwrap_or_else(SchemaIndex::empty);
            let schema = base_schema
                .overlay_for_open_documents(&documents)
                .unwrap_or_else(|| base_schema.base());
            let open = effective_open_documents(
                &saved,
                &documents,
                &schema,
                &id,
                retired_all.clone(),
                retired_targets.clone(),
            );
            let owners: BTreeMap<_, _> = open
                .iter()
                .map(|(key, document)| (key.clone(), document.identity().uri.clone()))
                .collect();
            let mut input_fingerprint = partition_input_fingerprint(
                &saved,
                &documents,
                &id,
                &schema,
                &owners,
                &retired_all,
                &retired_targets,
            );
            input_fingerprint.project_complete = saved.partition_is_complete(&id);
            let old = self.partitions.get(&id);
            let reusable = old.is_some_and(|old| old.input_fingerprint == input_fingerprint);
            let build_id = if reusable {
                old.map_or(next_partition_build_id, |old| old.build_id)
            } else {
                let build_id = next_partition_build_id.checked_add(1).ok_or(
                    AuthoringError::GenerationExhausted {
                        current: recite_compiler::authoring::SnapshotGeneration::new(
                            next_partition_build_id,
                        ),
                    },
                )?;
                next_partition_build_id = build_id;
                build_id
            };
            let kernel = if reusable {
                old.map(|old| Arc::clone(&old.kernel)).unwrap_or_default()
            } else {
                let fresh;
                let base = if let Some(old) = old.filter(|old| {
                    old.schema.schema() == schema.schema()
                        && old
                            .open_owners
                            .iter()
                            .all(|(key, uri)| owners.get(key).is_none_or(|next| next == uri))
                }) {
                    old.kernel.as_ref()
                } else {
                    fresh = schema
                        .schema()
                        .cloned()
                        .map(AuthoringKernel::with_schema)
                        .unwrap_or_default();
                    &fresh
                };
                let request = super::kernel::authoring_request(
                    &saved,
                    &open,
                    &id,
                    base.snapshot().generation(),
                )
                .with_project_completeness(saved.partition_is_complete(&id));
                Arc::new(base.updated(request, &self.control)?)
            };
            let retired_schema_uris = retired.get(&id).cloned().unwrap_or_default();
            partitions.insert(
                id,
                KernelPartition {
                    kernel,
                    build_id,
                    schema,
                    open_owners: owners,
                    retired_schema_uris,
                    input_fingerprint,
                },
            );
        }
        let snapshot = self
            .snapshot
            .rebuild(generation, &saved, &documents, &partitions);
        self.control.checkpoint()?;
        self.saved = saved;
        self.documents = documents;
        self.partitions = partitions;
        self.retired_schema_targets = retired_schema_targets;
        self.generation = generation;
        self.next_partition_build_id = next_partition_build_id;
        self.rebuild_query_index();
        self.snapshot = snapshot;
        Ok(())
    }
}

fn partition_input_fingerprint(
    saved: &SavedProjectIndex,
    documents: &OpenDocumentStore,
    partition: &str,
    schema: &SchemaIndex,
    owners: &BTreeMap<recite_core::DocumentKey, lsp_types::Uri>,
    retired: &BTreeSet<String>,
    retired_targets: &BTreeSet<String>,
) -> PartitionInputFingerprint {
    let saved = saved
        .documents
        .values()
        .filter(|document| {
            saved
                .partition_for_path(&document.identity.canonical_path)
                .as_deref()
                == Some(partition)
        })
        .map(|document| {
            (
                document.identity.project_relative_path.clone(),
                InputText(Arc::clone(&document.text)),
            )
        })
        .collect();
    let open = documents
        .documents()
        .filter_map(|document| {
            let key = super::document_key_for_open(document)?;
            if owners.get(&key) != Some(&document.identity().uri) {
                return None;
            }
            Some((
                key.as_str().to_owned(),
                document.identity().uri.as_str().to_owned(),
                document.version(),
                InputText(document.shared_text()),
            ))
        })
        .collect();
    PartitionInputFingerprint {
        saved,
        open,
        schema: schema.clone(),
        retired: retired.clone(),
        retired_targets: retired_targets.clone(),
        project_complete: false,
    }
}

#[path = "kernel_rebuild/tests.rs"]
#[cfg(test)]
mod tests;
