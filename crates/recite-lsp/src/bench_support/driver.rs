use std::path::PathBuf;

use lsp_types::{
    CompletionResponse, GotoDefinitionResponse, TextDocumentContentChangeEvent, WorkspaceEdit,
};
use recite_ui::{UiCatalog, UiLocale};

use super::memory::LspMemoryReport;
use super::probes::{
    LspBenchmarkProbes, LspDocumentProbe, LspPositionProbe, read_probe_text_or_panic,
};
use crate::diagnostics::publish_diagnostics;
use crate::workspace::{DiagnosticRefresh, LspWorkspace, WorkspaceChangeResult, WorkspaceConfig};

#[derive(Clone, Debug)]
pub struct LspBenchmarkConfig {
    roots: Vec<PathBuf>,
    schema_path: Option<PathBuf>,
}

impl LspBenchmarkConfig {
    #[must_use]
    pub fn new(roots: Vec<PathBuf>) -> Self {
        Self {
            roots,
            schema_path: None,
        }
    }

    #[must_use]
    pub fn with_schema_path(mut self, schema_path: PathBuf) -> Self {
        self.schema_path = Some(schema_path);
        self
    }

    fn workspace_config(&self) -> WorkspaceConfig {
        let mut config = WorkspaceConfig::for_roots(self.roots.clone());
        if let Some(schema_path) = &self.schema_path {
            config = config.with_schema_path(schema_path.clone());
        }
        config
    }
}

pub struct LspBenchmarkDriver {
    workspace: LspWorkspace,
}

impl LspBenchmarkDriver {
    #[must_use]
    pub fn new(config: &LspBenchmarkConfig) -> Self {
        let catalog = match UiCatalog::load(&UiLocale::default()) {
            Ok(catalog) => catalog,
            Err(error) => panic!("benchmark default UI catalog is invalid: {error}"),
        };
        Self {
            workspace: LspWorkspace::with_ui_catalog(config.workspace_config(), catalog),
        }
    }

    #[must_use]
    pub fn probes(&self) -> LspBenchmarkProbes {
        LspBenchmarkProbes::discover(&self.workspace)
    }

    #[must_use]
    pub fn memory_report(&self) -> LspMemoryReport {
        LspMemoryReport::from_workspace(&self.workspace)
    }

    #[must_use]
    pub fn open_file(&mut self, probe: &LspDocumentProbe) -> usize {
        let refresh = self
            .workspace
            .open(probe.uri.clone(), 1, read_probe_text_or_panic(probe));
        diagnostic_count(refresh)
    }

    #[must_use]
    pub fn change_file(&mut self, probe: &LspDocumentProbe) -> usize {
        self.workspace
            .open(probe.uri.clone(), 1, read_probe_text_or_panic(probe));
        match self.workspace.change(
            probe.uri.clone(),
            2,
            vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: read_probe_text_or_panic(probe),
            }],
        ) {
            WorkspaceChangeResult::Accepted(refresh) => diagnostic_count(refresh),
            other => panic!("LSP benchmark change was not accepted: {other:?}"),
        }
    }

    #[must_use]
    pub fn diagnostics_refresh(&mut self, probe: &LspDocumentProbe) -> usize {
        // Keep the synthetic refresh at the latest probe version so repeated
        // benchmark operations satisfy the kernel's monotonic overlay guard.
        let refresh = self
            .workspace
            .open(probe.uri.clone(), 2, read_probe_text_or_panic(probe));
        let DiagnosticRefresh::Publish(diagnostics) = refresh else {
            return 0;
        };
        let text = read_probe_text_or_panic(probe);
        publish_diagnostics(
            diagnostics.uri,
            &text,
            diagnostics.version,
            &diagnostics.diagnostics,
            &self.workspace.ui_catalog,
            &self.workspace.diagnostic_sources(),
        )
        .unwrap_or_else(|error| panic!("LSP benchmark diagnostic publication failed: {error}"))
        .diagnostics
        .len()
    }

    #[must_use]
    pub fn completion(&self, probe: &LspPositionProbe) -> Option<CompletionResponse> {
        self.workspace.completion(&probe.uri, probe.position)
    }

    #[must_use]
    pub fn definition(&self, probe: &LspPositionProbe) -> Option<GotoDefinitionResponse> {
        self.workspace.definition(&probe.uri, probe.position)
    }

    #[must_use]
    pub fn rename(&self, probe: &LspPositionProbe, new_name: &str) -> Option<WorkspaceEdit> {
        self.workspace.rename(&probe.uri, probe.position, new_name)
    }

    #[must_use]
    pub fn stale_change_is_suppressed(&mut self, probe: &LspDocumentProbe) -> bool {
        self.workspace
            .open(probe.uri.clone(), 2, read_probe_text_or_panic(probe));
        let generation = self.workspace.generation();
        let result = self.workspace.change(
            probe.uri.clone(),
            1,
            vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text: ":: stale\n> stale\n".to_owned(),
            }],
        );

        matches!(result, WorkspaceChangeResult::Stale)
            && self.workspace.is_current_generation(generation)
    }
}

fn diagnostic_count(refresh: DiagnosticRefresh) -> usize {
    match refresh {
        DiagnosticRefresh::Publish(diagnostics) => diagnostics.diagnostics.len(),
        DiagnosticRefresh::Clear { .. } => 0,
    }
}
