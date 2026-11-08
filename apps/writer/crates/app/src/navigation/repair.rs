//! External source repair shares navigation's cancellation, origin checks and worker retirement.
use super::*;
use recite_writer_model::{PreparedSourceRepair, SourceRepairRequest};
use recite_writer_session::project::save::ordered;
use recite_writer_session::project::{SourceRepairArtifact, SourceRepairExport};

#[derive(Default)]
pub struct Repair {
    document: Option<recite_core::DocumentKey>,
    pub(crate) path: String,
    pub(crate) native_path: Option<PathBuf>,
    pub artifact: Option<SourceRepairArtifact>,
}
#[derive(Clone)]
pub enum Action {
    Export,
    Import(PathBuf),
    Choose,
}
pub enum Work {
    Export(SourceRepairExport),
    Import {
        path: PathBuf,
        request: SourceRepairRequest,
    },
    Choose(fn() -> Option<PathBuf>),
}
pub enum ResultValue {
    Export(SourceRepairArtifact),
    Import(Box<PreparedSourceRepair>),
    Chosen(Option<PathBuf>),
}
impl Work {
    pub fn execute(self, control: &dyn WorkControl) -> Result<Box<ResultValue>, String> {
        control.checkpoint().map_err(|error| error.to_string())?;
        let value = match self {
            Self::Export(export) => {
                ResultValue::Export(export.write().map_err(|error| error.to_string())?)
            }
            Self::Import { path, request } => {
                let source = recite_writer_session::read_regular(&path)
                    .map_err(|error| error.to_string())?;
                ResultValue::Import(Box::new(
                    request
                        .prepare(source.into(), control)
                        .map_err(|error| error.to_string())?,
                ))
            }
            Self::Choose(choose) => ResultValue::Chosen(choose()),
        };
        Ok(Box::new(value))
    }
}
impl Writer {
    pub(crate) fn source_repair_requirement(&self, scope: &ordered::Scope) -> Option<PathBuf> {
        use recite_writer_session::project::save::ordered::Scope;
        let project = self.session.project()?;
        match scope {
            Scope::AcceptedCurrent | Scope::Declarations => return None,
            Scope::Current => {
                return self.source_repair_pending().then(|| {
                    project
                        .current()
                        .path()
                        .strip_prefix(project.root())
                        .unwrap_or(project.current().path())
                        .to_owned()
                });
            }
            Scope::All | Scope::Document(_) => {}
        }
        project.sessions().find_map(|session| {
            let affected = match scope {
                Scope::Current => session.path() == project.current().path(),
                Scope::All => true,
                Scope::Document(path) => session.path() == path,
                Scope::AcceptedCurrent | Scope::Declarations => false,
            };
            let rules_mounted = session.path() == project.current().path() && self.rules.is_some();
            (affected
                && !rules_mounted
                && session.model().view() == &View::Source
                && session.model().has_draft())
            .then(|| {
                session
                    .path()
                    .strip_prefix(project.root())
                    .unwrap_or(session.path())
                    .to_owned()
            })
        })
    }
    pub fn source_repair_pending(&self) -> bool {
        self.session.model().view() == &View::Source
            && self.session.model().has_draft()
            && self.rules.is_none()
    }
    pub fn repair_path(&self) -> &str {
        if self.repair.document.as_ref() == Some(self.session.model().document().key()) {
            &self.repair.path
        } else {
            ""
        }
    }
    pub fn edit_repair_path(&mut self, path: String) {
        self.repair.document = Some(self.session.model().document().key().clone());
        self.repair.path = path;
        self.repair.native_path = None;
    }
    pub fn export_source_repair(&mut self) {
        self.begin_source_repair(Action::Export);
    }
    pub fn choose_source_repair(&mut self) {
        self.begin_source_repair(Action::Choose);
    }
    pub fn import_source_repair(&mut self) {
        if self.repair_path().trim().is_empty() {
            self.status = "Choose the repaired source file for this document first.".into();
            return;
        }
        let path = self
            .repair
            .native_path
            .clone()
            .unwrap_or_else(|| PathBuf::from(self.repair.path.trim()));
        self.begin_source_repair(Action::Import(path));
    }
    fn begin_source_repair(&mut self, action: Action) {
        let result = self.submit_source_repair(action);
        if let Err(error) = result {
            self.status = error;
        }
    }
    fn submit_source_repair(&mut self, action: Action) -> Result<(), String> {
        if self.work.busy() || self.navigation.busy() || self.phase != crate::Phase::Writing {
            return Err("Finish the current editor action before repairing source.".into());
        }
        if self.session.model().view() != &View::Source {
            return Err("Open the recovered source draft before importing its repair.".into());
        }
        let work = match &action {
            Action::Export => {
                let project = self
                    .session
                    .project()
                    .ok_or("Open a project to export its source draft.")?;
                Work::Export(
                    project
                        .files()
                        .source_repair_export(self.session.model())
                        .map_err(|error| error.to_string())?,
                )
            }
            Action::Import(path) => Work::Import {
                path: path.clone(),
                request: self
                    .session
                    .model()
                    .source_repair_request()
                    .map_err(|error| error.to_string())?,
            },
            Action::Choose => Work::Choose(self.host.choose_repaired_source),
        };
        let request = Arc::new(Request {
            target: Target::Repair(action),
            origin: Origin::capture(self),
            cancel: CancellationToken::new(),
            forwarded: None,
        });
        self.send_navigation(request, worker::Work::Repair(Box::new(work)))
    }
    pub(super) fn publish_source_repair(
        &mut self,
        request: &Request,
        value: &mut ResultValue,
    ) -> Result<(), String> {
        if let ResultValue::Export(artifact) = value {
            // Reconcile the durable export even when cancellation or newer typing vetoes launch.
            self.repair.path = artifact.source.to_string_lossy().into_owned();
            self.repair.document = Some(self.session.model().document().key().clone());
            self.repair.native_path = Some(artifact.source.clone());
            self.recovery_copy = Some(artifact.recovery.clone());
            self.repair.artifact = Some(artifact.clone());
            if request.checkpoint().is_ok() && request.origin.is_current(self) {
                self.open_exported_source();
            } else {
                self.status = "Exported the captured draft and recovery backup. Current work stays open; choose Open exported source to inspect it.".into();
            }
            return Ok(());
        }
        request
            .checkpoint()
            .map_err(|_| "Source repair cancelled; your current draft stays open.")?;
        if !request.origin.is_current(self) {
            return Err("The draft changed while source repair was preparing. Your newer work stays open; retry import.".into());
        }
        match value {
            ResultValue::Import(prepared) => {
                self.session
                    .edit(|model| model.accept_source_repair(prepared))
                    .map_err(|error| error.to_string())?;
                self.retire_rules();
                self.location.screen = recite_writer_session::location::Screen::Write;
                self.location.source = true;
                self.location.passage = None;
                self.location.beat = None;
                self.reading.prepare(self.session.model());
                self.invalidate_map();
                self.status = "Repaired source accepted. Save file to write it to disk.".into();
            }
            ResultValue::Chosen(path) => {
                if let Some(path) = path {
                    self.repair.document = Some(self.session.model().document().key().clone());
                    self.repair.path = path.to_string_lossy().into_owned();
                    self.repair.native_path = Some(path.clone());
                }
                self.status = "Choose Import repaired source to validate the selected text.".into();
            }
            ResultValue::Export(_) => {}
        }
        Ok(())
    }
}
