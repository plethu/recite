use recite_ui::{UiCatalog, UiLocale};

use super::SnapshotGeneration;
use super::project_index::{LiveProjectSnapshot, SavedProjectIndex};
use super::schema_index::SchemaIndex;
use super::{DiagnosticRefresh, LspWorkspace, WorkspaceConfig};
use crate::diagnostics::DiagnosticSource;
use crate::documents::OpenDocumentStore;

impl LspWorkspace {
    #[allow(dead_code, reason = "used by unit tests and benchmark support")]
    pub(crate) fn new(config: WorkspaceConfig) -> Self {
        Self::with_ui_catalog(config, default_ui_catalog())
    }

    pub(crate) fn with_ui_catalog(config: WorkspaceConfig, ui_catalog: UiCatalog) -> Self {
        let saved = SavedProjectIndex::discover(&config);
        let schema = SchemaIndex::load(config.schema_path);
        let documents = OpenDocumentStore::default();
        let generation = SnapshotGeneration(0);
        let snapshot = LiveProjectSnapshot::rebuild(generation, &saved, &documents);

        Self {
            saved,
            documents,
            snapshot,
            schema,
            generation,
            ui_catalog,
        }
    }

    pub(crate) fn diagnostic_sources(&self) -> Vec<DiagnosticSource<'_>> {
        self.snapshot
            .summaries()
            .iter()
            .filter_map(|summary| {
                let path = summary
                    .project_relative_path()
                    .unwrap_or_else(|| summary.uri().as_str());
                Some(DiagnosticSource {
                    path,
                    uri: summary.uri(),
                    text: self.text_for_summary(summary)?,
                })
            })
            .collect()
    }

    pub(crate) fn project_diagnostics(&self) -> Option<DiagnosticRefresh> {
        let uri = self
            .saved
            .manifest_path()
            .and_then(crate::paths::file_path_to_uri)?;
        if self.saved.diagnostics().is_empty() {
            return None;
        }
        Some(DiagnosticRefresh::Publish(super::DocumentDiagnostics {
            uri,
            text: self.saved.manifest_text().to_owned(),
            version: None,
            diagnostics: self.saved.diagnostics().to_vec(),
            generation: self.generation,
        }))
    }
}

#[allow(
    dead_code,
    clippy::expect_used,
    reason = "used by the default workspace constructor"
)]
fn default_ui_catalog() -> UiCatalog {
    UiCatalog::load(&UiLocale::default()).expect("embedded default UI catalog must load")
}
