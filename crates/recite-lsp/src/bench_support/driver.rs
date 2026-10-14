use std::path::PathBuf;

use lsp_types::{
    CodeActionParams, CodeActionResponse, CompletionResponse, GotoDefinitionResponse, Hover,
    Location, PublishDiagnosticsParams, TextDocumentContentChangeEvent, WorkspaceEdit,
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
            workspace: LspWorkspace::with_ui_catalog(config.workspace_config(), catalog)
                .unwrap_or_else(|error| panic!("benchmark authoring state is invalid: {error}")),
        }
    }

    #[must_use]
    pub fn probes(&self) -> LspBenchmarkProbes {
        LspBenchmarkProbes::discover(&self.workspace)
    }

    /// Assert that discovered query positions return meaningful responses.
    /// Opens the completion document and evaluates requests during untimed setup.
    pub fn assert_query_probes(&mut self, probes: &LspBenchmarkProbes) {
        let document = LspDocumentProbe {
            uri: probes.completion.uri.clone(),
            path: probes.completion.path.clone(),
            project_relative_path: probes.completion.project_relative_path.clone(),
        };
        let _ = self.open_file(&document);
        let completion_has_items = match self.completion(&probes.completion) {
            Some(CompletionResponse::Array(items)) => !items.is_empty(),
            Some(CompletionResponse::List(list)) => !list.items.is_empty(),
            None => false,
        };
        assert!(
            completion_has_items,
            "LSP benchmark completion probe returns items"
        );
        let definition_has_target = match self.definition(&probes.definition) {
            Some(GotoDefinitionResponse::Scalar(_)) => true,
            Some(GotoDefinitionResponse::Array(locations)) => !locations.is_empty(),
            Some(GotoDefinitionResponse::Link(links)) => !links.is_empty(),
            None => false,
        };
        assert!(
            definition_has_target,
            "LSP benchmark definition probe resolves a target"
        );
    }

    #[must_use]
    pub fn memory_report(&self) -> LspMemoryReport {
        LspMemoryReport::from_workspace(&self.workspace)
    }

    #[must_use]
    pub fn open_file(&mut self, probe: &LspDocumentProbe) -> usize {
        let Some(refresh) = self
            .workspace
            .open_refreshes(probe.uri.clone(), 1, read_probe_text_or_panic(probe))
            .into_iter()
            .next()
        else {
            return 0;
        };
        diagnostic_count(refresh)
    }

    #[must_use]
    pub fn change_file(&mut self, probe: &LspDocumentProbe) -> usize {
        // Exercise a real local edit, rather than only advancing the version of
        // byte-identical input. Structural/prose variants live in the stdio suite.
        let mut text = read_probe_text_or_panic(probe);
        text.push_str("\n# benchmark local edit\n");
        match self.workspace.change(
            probe.uri.clone(),
            2,
            vec![TextDocumentContentChangeEvent {
                range: None,
                range_length: None,
                text,
            }],
        ) {
            WorkspaceChangeResult::Accepted(refresh) => diagnostic_count(refresh),
            other => panic!("LSP benchmark change was not accepted: {other:?}"),
        }
    }

    #[must_use]
    pub fn diagnostics_refresh(&mut self, probe: &LspDocumentProbe) -> usize {
        self.workspace
            .open_refreshes(probe.uri.clone(), 2, read_probe_text_or_panic(probe));
        self.diagnostics_publication(probe)
            .map_or(0, |params| params.diagnostics.len())
    }

    #[must_use]
    pub fn diagnostics_publication(
        &self,
        probe: &LspDocumentProbe,
    ) -> Option<PublishDiagnosticsParams> {
        // Project the requested document's prepared snapshot, rather than an
        // unrelated first refresh or a suppressed byte-identical reopen.
        let refresh = self.workspace.diagnostic_refresh_for_uri(&probe.uri);
        let DiagnosticRefresh::Publish(diagnostics) = refresh else {
            return None;
        };
        Some(
            publish_diagnostics(
                diagnostics.uri.clone(),
                &diagnostics.text,
                diagnostics.version,
                &diagnostics.diagnostics,
                &self.workspace.ui_catalog,
                &self
                    .workspace
                    .diagnostic_related_sources(&diagnostics.uri, &diagnostics.diagnostics),
            )
            .unwrap_or_else(|error| panic!("LSP benchmark diagnostic publication failed: {error}")),
        )
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
    pub fn hover(&self, probe: &LspPositionProbe) -> Option<Hover> {
        self.workspace
            .hover_with_control(&probe.uri, probe.position, &Default::default())
    }

    #[must_use]
    pub fn references(&self, probe: &LspPositionProbe) -> Option<Vec<Location>> {
        self.workspace.references_with_control(
            &probe.uri,
            probe.position,
            true,
            &Default::default(),
        )
    }

    #[must_use]
    pub fn code_action(&self, params: &CodeActionParams) -> Option<CodeActionResponse> {
        self.workspace
            .code_action_with_control(params, &Default::default())
    }

    #[must_use]
    pub fn fork(&self) -> Self {
        Self {
            workspace: self.workspace.clone(),
        }
    }

    #[must_use]
    pub fn rename(&self, probe: &LspPositionProbe, new_name: &str) -> Option<WorkspaceEdit> {
        self.workspace.rename(&probe.uri, probe.position, new_name)
    }

    #[must_use]
    pub fn stale_change_is_suppressed(&mut self, probe: &LspDocumentProbe) -> bool {
        self.workspace
            .open_refreshes(probe.uri.clone(), 2, read_probe_text_or_panic(probe));
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
