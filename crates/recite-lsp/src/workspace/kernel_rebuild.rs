use std::collections::{BTreeMap, BTreeSet};

use recite_compiler::AuthoringKernel;

use super::kernel::{KernelPartition, effective_open_documents};
use super::partition_rollback::take_old_partitions;
use super::project_index::SavedProjectIndex;
use super::schema_index::SchemaIndex;
use super::{LspWorkspace, SnapshotGeneration};
use crate::documents::OpenDocumentStore;

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct PartitionInputFingerprint {
    saved: Vec<(String, String)>,
    open: Vec<(String, String, i32, String)>,
    schema: SchemaIndex,
    retired: BTreeSet<String>,
    retired_targets: BTreeSet<String>,
}

impl LspWorkspace {
    pub(super) fn rebuild_for_documents_with_schemas(
        &mut self,
        saved: SavedProjectIndex,
        documents: OpenDocumentStore,
        schemas: BTreeMap<String, SchemaIndex>,
    ) -> Result<(), recite_compiler::AuthoringError> {
        let retired = self
            .partitions
            .iter()
            .map(|(id, partition)| (id.clone(), partition.retired_schema_uris.clone()))
            .collect();
        let old_partitions = std::mem::take(&mut self.partitions);
        match self.rebuild_partitions(saved, documents, schemas, retired, Some(old_partitions)) {
            Ok(()) => Ok(()),
            Err((error, old_partitions)) => {
                self.partitions = old_partitions;
                Err(error)
            }
        }
    }

    pub(super) fn rebuild_for_documents_with_schemas_and_retired(
        &mut self,
        saved: SavedProjectIndex,
        documents: OpenDocumentStore,
        schemas: BTreeMap<String, SchemaIndex>,
        retired: BTreeMap<String, BTreeSet<String>>,
    ) -> Result<(), recite_compiler::AuthoringError> {
        let old_partitions = std::mem::take(&mut self.partitions);
        match self.rebuild_partitions(saved, documents, schemas, retired, Some(old_partitions)) {
            Ok(()) => Ok(()),
            Err((error, old_partitions)) => {
                self.partitions = old_partitions;
                Err(error)
            }
        }
    }

    fn rebuild_partitions(
        &mut self,
        saved: SavedProjectIndex,
        documents: OpenDocumentStore,
        schemas: BTreeMap<String, SchemaIndex>,
        retired: BTreeMap<String, BTreeSet<String>>,
        mut old_partitions: Option<BTreeMap<String, KernelPartition>>,
    ) -> Result<
        (),
        (
            recite_compiler::AuthoringError,
            BTreeMap<String, KernelPartition>,
        ),
    > {
        let generation = SnapshotGeneration(
            self.generation
                .0
                .checked_add(1)
                .ok_or(recite_compiler::AuthoringError::GenerationExhausted {
                    current: recite_compiler::SnapshotGeneration::new(self.generation.0),
                })
                .map_err(|error| (error, take_old_partitions(&mut old_partitions)))?,
        );
        let mut next_partition_build_id = self.next_partition_build_id;
        let mut ids = saved.partition_ids();
        ids.extend(schemas.keys().cloned());
        let mut retired_all = retired
            .values()
            .flat_map(|uris| uris.iter().cloned())
            .chain(self.retired_schema_uris.iter().cloned())
            .chain(self.retired_schema_targets.keys().cloned())
            .collect::<BTreeSet<_>>();
        let retired_targets = self
            .retired_schema_targets
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
            let base_schema = schemas.get(&id).cloned().unwrap_or_else(SchemaIndex::empty);
            let schema = base_schema
                .overlay_for_documents_in_partition(&documents, &saved, &id)
                .or_else(|| {
                    base_schema
                        .has_open_match_in_partition(&documents, &saved, &id)
                        .then(|| {
                            documents
                                .documents()
                                .find(|document| base_schema.matches_uri(&document.identity().uri))
                                .map(|document| {
                                    base_schema.unavailable_overlay(document.identity().uri.clone())
                                })
                        })
                        .flatten()
                })
                .unwrap_or_else(|| base_schema.base());
            let open = effective_open_documents(
                &saved,
                &documents,
                &schema,
                &id,
                retired_all.clone(),
                retired_targets.clone(),
            );
            let owners = open
                .iter()
                .map(|(key, document)| (key.clone(), document.identity().uri.clone()))
                .collect();
            let input_fingerprint = partition_input_fingerprint(
                &saved,
                &documents,
                &id,
                &schema,
                &owners,
                &retired_all,
                &retired_targets,
            );
            let reusable = old_partitions
                .as_ref()
                .and_then(|old| old.get(&id))
                .is_some_and(|old| old.input_fingerprint == input_fingerprint);
            let build_id = if reusable {
                old_partitions
                    .as_ref()
                    .and_then(|old| old.get(&id))
                    .map_or(next_partition_build_id, |old| old.build_id)
            } else {
                let build_id = next_partition_build_id
                    .checked_add(1)
                    .ok_or(recite_compiler::AuthoringError::GenerationExhausted {
                        current: recite_compiler::SnapshotGeneration::new(next_partition_build_id),
                    })
                    .map_err(|error| (error, take_old_partitions(&mut old_partitions)))?;
                next_partition_build_id = build_id;
                build_id
            };
            let mut kernel = schema
                .schema()
                .cloned()
                .map(AuthoringKernel::with_schema)
                .unwrap_or_default();
            if !reusable {
                let expected = kernel.snapshot().generation();
                let request = super::kernel::authoring_request(&saved, &open, &id, expected);
                kernel
                    .apply(request)
                    .map_err(|error| (error, take_old_partitions(&mut old_partitions)))?;
            }
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
        let mut old_partitions = take_old_partitions(&mut old_partitions);
        for (id, partition) in &mut partitions {
            if let Some(old) = old_partitions.remove(id)
                && old.input_fingerprint == partition.input_fingerprint
            {
                partition.kernel = old.kernel;
            }
        }
        let snapshot = super::snapshot::LiveProjectSnapshot::rebuild(
            generation,
            &saved,
            &documents,
            &partitions,
        );
        self.saved = saved;
        self.documents = documents;
        self.partitions = partitions;
        self.generation = generation;
        self.next_partition_build_id = next_partition_build_id;
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
                document.text.clone(),
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
                document.text().to_owned(),
            ))
        })
        .collect();
    PartitionInputFingerprint {
        saved,
        open,
        schema: schema.clone(),
        retired: retired.clone(),
        retired_targets: retired_targets.clone(),
    }
}
