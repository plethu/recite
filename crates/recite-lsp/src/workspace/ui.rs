use recite_compiler::AuthoringKernel;
use recite_ui::{UiCatalog, UiLocale};

use super::SnapshotGeneration;
use super::project_index::SavedProjectIndex;
use super::schema_index::SchemaIndex;
use super::snapshot::LiveProjectSnapshot;
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
        let kernel = schema
            .schema()
            .cloned()
            .map_or_else(AuthoringKernel::new, AuthoringKernel::with_schema);
        let generation = SnapshotGeneration(0);
        let mut workspace = LspWorkspace {
            saved,
            documents,
            kernel,
            snapshot: LiveProjectSnapshot::empty(generation),
            schema,
            generation,
            ui_catalog,
        };
        workspace.rebuild_kernel();
        workspace.snapshot = LiveProjectSnapshot::rebuild(
            generation,
            &workspace.saved,
            &workspace.documents,
            workspace.kernel.snapshot(),
        );
        workspace
    }

    pub(crate) fn diagnostic_sources(&self) -> Vec<DiagnosticSource<'_>> {
        self.snapshot
            .summaries()
            .iter()
            .filter_map(|summary| {
                let path = super::document_key_for_identity(&summary.identity)?;
                Some(DiagnosticSource {
                    path: path.as_str().to_owned(),
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

#[allow(dead_code, reason = "used by the default workspace constructor")]
fn default_ui_catalog() -> UiCatalog {
    #[expect(
        clippy::expect_used,
        reason = "the embedded default UI catalog is validated by the UI contract gate"
    )]
    {
        UiCatalog::load(&UiLocale::default()).expect("embedded default UI catalog must load")
    }
}
