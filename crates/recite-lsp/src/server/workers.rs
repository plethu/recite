use super::{freshness::Epochs, query::Query, updates::Update};
use crate::{
    diagnostics::{clear_diagnostics, publish_diagnostics},
    workspace::{DiagnosticRefresh, LspWorkspace, WorkspaceConfig},
};
use crossbeam_channel::{Receiver, bounded};
use lsp_types::{InitializeParams, PublishDiagnosticsParams, Uri};
use recite_compiler::authoring::CancellationToken;
use recite_ui::UiCatalog;
use std::{
    collections::BTreeMap,
    sync::{
        Arc,
        mpsc::{SyncSender, sync_channel},
    },
    thread::{self, JoinHandle},
};

pub(super) struct AnalysisJob {
    pub(super) through: u64,
    pub(super) epochs: Epochs,
    pub(super) updates: Vec<Arc<Update>>,
    pub(super) refresh_uris: Vec<Uri>,
    pub(super) control: CancellationToken,
}
pub(super) struct AnalysisResult {
    pub(super) through: u64,
    pub(super) epochs: Epochs,
    pub(super) result: Result<Option<AnalysisSnapshot>, String>,
}
pub(super) struct AnalysisSnapshot {
    pub(super) workspace: Arc<LspWorkspace>,
    pub(super) scopes: BTreeMap<String, String>,
    pub(super) diagnostics: Vec<PublishDiagnosticsParams>,
}
pub(super) struct QueryJob {
    pub(super) serial: u64,
    pub(super) query: Query,
    pub(super) workspace: Arc<LspWorkspace>,
    pub(super) control: CancellationToken,
}
pub(super) struct QueryResult {
    pub(super) serial: u64,
    pub(super) result: Result<serde_json::Value, String>,
}

pub(super) struct Workers {
    pub(super) analysis: SyncSender<AnalysisJob>,
    pub(super) analyzed: Receiver<AnalysisResult>,
    pub(super) query: SyncSender<QueryJob>,
    pub(super) queried: Receiver<QueryResult>,
    analysis_thread: JoinHandle<()>,
    query_thread: JoinHandle<()>,
}
impl Workers {
    pub(super) fn start(params: InitializeParams, catalog: UiCatalog) -> Self {
        let (analysis, inputs) = sync_channel::<AnalysisJob>(1);
        let (outputs, analyzed) = bounded(1);
        let analysis_thread = thread::spawn(move || {
            let catalog = Arc::new(catalog);
            let mut cached: Option<Arc<LspWorkspace>> = None;
            // Each input has one consumer. Keep capacity one and use the
            // standard blocking receive; results remain selectable Crossbeam channels.
            while let Ok(job) = inputs.recv() {
                tracing::trace!(phase = "analysis_start", revision = job.through);
                let result = analyze(&params, &catalog, cached.as_deref(), &job);
                tracing::trace!(phase = "analysis_end", revision = job.through);
                if let Ok(Some(snapshot)) = &result {
                    cached = Some(Arc::clone(&snapshot.workspace));
                }
                if outputs
                    .send(AnalysisResult {
                        through: job.through,
                        epochs: job.epochs,
                        result,
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        let (query, inputs) = sync_channel::<QueryJob>(1);
        let (outputs, queried) = bounded(1);
        let query_thread = thread::spawn(move || {
            while let Ok(job) = inputs.recv() {
                tracing::trace!(phase = "query_start", serial = job.serial);
                let result = job.query.execute(&job.workspace, &job.control);
                tracing::trace!(phase = "query_end", serial = job.serial);
                if outputs
                    .send(QueryResult {
                        serial: job.serial,
                        result,
                    })
                    .is_err()
                {
                    break;
                }
            }
        });
        Self {
            analysis,
            analyzed,
            query,
            queried,
            analysis_thread,
            query_thread,
        }
    }
    pub(super) fn join(self) -> Result<(), super::ServerError> {
        drop((self.analysis, self.query, self.analyzed, self.queried));
        let analysis = self.analysis_thread.join();
        let query = self.query_thread.join();
        if analysis.is_err() || query.is_err() {
            return Err(super::ServerError::WorkerPanic);
        }
        Ok(())
    }
}

fn analyze(
    params: &InitializeParams,
    catalog: &Arc<UiCatalog>,
    cached: Option<&LspWorkspace>,
    job: &AnalysisJob,
) -> Result<Option<AnalysisSnapshot>, String> {
    let outcome = (|| {
        job.control.checkpoint()?;
        let mut workspace = match cached {
            Some(workspace) => workspace.clone(),
            None => LspWorkspace::with_control(
                WorkspaceConfig::from_initialize_params(params),
                Arc::clone(catalog),
                job.control.clone(),
            )?,
        };
        workspace.control = job.control.clone();
        let mut diagnostics = Vec::new();
        if cached.is_none() {
            for refresh in workspace
                .project_diagnostics_all()
                .into_iter()
                .chain(workspace.schema_diagnostics_all())
            {
                replace_diagnostic(&mut diagnostics, project_refresh(&workspace, refresh)?);
            }
        }
        for update in &job.updates {
            job.control.checkpoint()?;
            let refreshes = update.apply(&mut workspace);
            job.control.checkpoint()?;
            if !update.is_applied(&workspace) {
                return Err(AnalysisError::RejectedInput);
            }
            for refresh in refreshes {
                job.control.checkpoint()?;
                if !workspace.is_current_generation(refresh.generation()) {
                    continue;
                }
                let params = project_refresh(&workspace, refresh)?;
                replace_diagnostic(&mut diagnostics, params);
            }
        }
        job.control.checkpoint()?;
        let mut refreshed = Vec::new();
        for uri in &job.refresh_uris {
            job.control.checkpoint()?;
            replace_diagnostic(
                &mut refreshed,
                project_refresh(&workspace, workspace.diagnostic_refresh_for_uri(uri))?,
            );
        }
        for params in diagnostics {
            replace_diagnostic(&mut refreshed, params);
        }
        let scopes = workspace.query_scopes();
        Ok::<_, AnalysisError>(AnalysisSnapshot {
            workspace: Arc::new(workspace),
            scopes,
            diagnostics: refreshed,
        })
    })();
    if job.control.checkpoint().is_err() {
        return Ok(None);
    }
    outcome.map(Some).map_err(|error| error.to_string())
}
#[derive(Debug, thiserror::Error)]
enum AnalysisError {
    #[error("accepted editor input was rejected by workspace analysis")]
    RejectedInput,
    #[error(transparent)]
    Interrupted(#[from] recite_compiler::authoring::Interrupted),
    #[error(transparent)]
    Authoring(#[from] recite_compiler::authoring::AuthoringError),
    #[error("diagnostic projection failed: {0}")]
    Projection(String),
}
fn project_refresh(
    workspace: &LspWorkspace,
    refresh: DiagnosticRefresh,
) -> Result<PublishDiagnosticsParams, AnalysisError> {
    match refresh {
        DiagnosticRefresh::Publish(d) => publish_diagnostics(
            d.uri.clone(),
            &d.text,
            d.version,
            &d.diagnostics,
            &workspace.ui_catalog,
            &workspace.diagnostic_sources_for_uri(&d.uri),
        )
        .map_err(|error| AnalysisError::Projection(error.to_string())),
        DiagnosticRefresh::Clear { uri, version, .. } => Ok(clear_diagnostics(uri, version)),
    }
}

fn replace_diagnostic(
    diagnostics: &mut Vec<PublishDiagnosticsParams>,
    params: PublishDiagnosticsParams,
) {
    if let Some(existing) = diagnostics
        .iter_mut()
        .find(|existing| existing.uri == params.uri)
    {
        *existing = params;
    } else {
        diagnostics.push(params);
    }
}
