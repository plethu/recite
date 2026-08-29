use recite_ui::{UiCatalog, UiLocale};

use super::SnapshotGeneration;
use super::project_index::{LiveProjectSnapshot, SavedProjectIndex};
use super::schema_index::SchemaIndex;
use super::{LspWorkspace, WorkspaceConfig};
use crate::diagnostics::DiagnosticSource;
use crate::documents::OpenDocumentStore;

impl LspWorkspace {
    pub(crate) fn new(config: WorkspaceConfig) -> Self {
        Self::with_ui_catalog(config, default_ui_catalog())
    }

    #[allow(dead_code)]
    pub(crate) fn with_ui_locale(
        config: WorkspaceConfig,
        locale: &UiLocale,
    ) -> Result<Self, String> {
        let catalog = UiCatalog::load(locale).map_err(|error| error.to_string())?;
        Ok(Self::with_ui_catalog(config, catalog))
    }

    pub(crate) fn with_ui_catalog(config: WorkspaceConfig, ui_catalog: UiCatalog) -> Self {
        let saved = SavedProjectIndex::discover(&config.roots);
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
}

#[allow(clippy::expect_used)]
fn default_ui_catalog() -> UiCatalog {
    UiCatalog::load(&UiLocale::default()).expect("embedded default UI catalog must load")
}
