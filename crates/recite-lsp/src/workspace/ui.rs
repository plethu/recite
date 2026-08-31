use recite_ui::UiCatalog;
use std::collections::{BTreeMap, BTreeSet};

use super::SnapshotGeneration;
use super::config::schema_paths_for_saved;
use super::project_index::SavedProjectIndex;
use super::schema_index::SchemaIndex;
use super::snapshot::LiveProjectSnapshot;
use super::{DiagnosticRefresh, LspWorkspace, WorkspaceConfig};
use crate::diagnostics::DiagnosticSource;
use crate::documents::OpenDocumentStore;

impl LspWorkspace {
    pub(crate) fn with_ui_catalog(
        config: WorkspaceConfig,
        ui_catalog: UiCatalog,
    ) -> Result<Self, recite_compiler::AuthoringError> {
        let saved = SavedProjectIndex::discover(&config);
        let schema_override_path = config.schema_override_path.clone();
        let documents = OpenDocumentStore::default();
        let schema_paths = schema_paths_for_saved(&saved, schema_override_path.as_ref());
        let mut schemas = BTreeMap::new();
        let partition_ids = saved.partition_ids();
        for id in partition_ids {
            let path = schema_paths.get(&id).cloned().flatten();
            schemas.insert(id, SchemaIndex::load(path));
        }
        let generation = SnapshotGeneration(0);
        let mut workspace = LspWorkspace {
            saved: saved.clone(),
            documents: documents.clone(),
            partitions: BTreeMap::new(),
            snapshot: LiveProjectSnapshot::empty(generation),
            schema_override_path,
            schema_paths,
            retired_schema_uris: BTreeSet::new(),
            generation,
            ui_catalog,
        };
        workspace.rebuild_for_documents_with_schemas(saved, documents, schemas)?;
        Ok(workspace)
    }

    #[cfg(any(test, feature = "bench-support"))]
    pub(crate) fn diagnostic_sources(&self) -> Vec<DiagnosticSource<'_>> {
        self.diagnostic_sources_for_partition(None)
    }

    pub(crate) fn diagnostic_sources_for_uri(
        &self,
        uri: &lsp_types::Uri,
    ) -> Vec<DiagnosticSource<'_>> {
        if self.is_schema_document_uri(uri) {
            return self.diagnostic_sources_for_partition(None);
        }
        self.diagnostic_sources_for_partition(self.partition_id_for_uri(uri).as_deref())
    }

    fn diagnostic_sources_for_partition(
        &self,
        partition: Option<&str>,
    ) -> Vec<DiagnosticSource<'_>> {
        self.snapshot
            .summaries()
            .iter()
            .filter_map(|summary| {
                if let Some(partition) = partition
                    && self.partition_id_for_uri(summary.uri()).as_deref() != Some(partition)
                {
                    return None;
                }
                let path = super::document_key_for_identity(&summary.identity)?;
                Some(DiagnosticSource {
                    path: path.as_str().to_owned(),
                    uri: summary.uri(),
                    text: self.text_for_summary(summary)?,
                })
            })
            .collect()
    }

    pub(crate) fn project_diagnostics_all(&self) -> Vec<DiagnosticRefresh> {
        self.saved
            .manifest_diagnostics()
            .values()
            .filter_map(|entry| {
                Some(DiagnosticRefresh::Publish(super::DocumentDiagnostics {
                    uri: crate::paths::file_path_to_uri(&entry.path)?,
                    text: entry.text.clone(),
                    version: None,
                    diagnostics: entry.diagnostics.clone(),
                    generation: self.generation,
                }))
            })
            .collect()
    }
}
