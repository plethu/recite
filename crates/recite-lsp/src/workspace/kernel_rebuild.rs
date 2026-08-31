use std::collections::{BTreeMap, BTreeSet};

use recite_compiler::AuthoringKernel;

use super::kernel::{KernelPartition, effective_open_documents};
use super::project_index::SavedProjectIndex;
use super::schema_index::SchemaIndex;
use super::{LspWorkspace, SnapshotGeneration};
use crate::documents::OpenDocumentStore;

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
        self.rebuild_partitions(saved, documents, schemas, retired)
    }

    pub(super) fn rebuild_for_documents_with_schemas_and_retired(
        &mut self,
        saved: SavedProjectIndex,
        documents: OpenDocumentStore,
        schemas: BTreeMap<String, SchemaIndex>,
        retired: BTreeMap<String, BTreeSet<String>>,
    ) -> Result<(), recite_compiler::AuthoringError> {
        self.rebuild_partitions(saved, documents, schemas, retired)
    }

    fn rebuild_partitions(
        &mut self,
        saved: SavedProjectIndex,
        documents: OpenDocumentStore,
        schemas: BTreeMap<String, SchemaIndex>,
        retired: BTreeMap<String, BTreeSet<String>>,
    ) -> Result<(), recite_compiler::AuthoringError> {
        let mut old_partitions = std::mem::take(&mut self.partitions);
        let mut ids = saved.partition_ids();
        ids.extend(schemas.keys().cloned());
        let retired_all = retired
            .values()
            .flat_map(|uris| uris.iter().cloned())
            .chain(self.retired_schema_uris.iter().cloned())
            .collect::<BTreeSet<_>>();
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
            let open =
                effective_open_documents(&saved, &documents, &schema, &id, retired_all.clone());
            let owners = open
                .iter()
                .map(|(key, document)| (key.clone(), document.identity().uri.clone()))
                .collect();
            let reusable = old_partitions.remove(&id).filter(|partition| {
                partition.schema.same_state(&schema) && partition.open_owners == owners
            });
            let mut kernel = reusable
                .map(|partition| partition.kernel)
                .or_else(|| schema.schema().cloned().map(AuthoringKernel::with_schema))
                .unwrap_or_default();
            let expected = kernel.snapshot().generation();
            let request = super::kernel::authoring_request(&saved, &open, &id, expected);
            kernel.apply(request)?;
            let retired_schema_uris = retired.get(&id).cloned().unwrap_or_default();
            partitions.insert(
                id,
                KernelPartition {
                    kernel,
                    schema,
                    open_owners: owners,
                    retired_schema_uris,
                },
            );
        }
        let generation = SnapshotGeneration(self.generation.0.saturating_add(1));
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
        self.snapshot = snapshot;
        Ok(())
    }
}
