//! One project-tool operation owns its terminal receipt across screen changes.
mod accepting;
mod declaration;
mod job;
pub mod project;
pub mod settings;
pub use super::{Phase, Writer, declarations::Row};
pub use project::Action as ProjectAction;
pub use project::ExternalAction;
use recite_writer_session::{
    declarations::{
        DeclarationAction, DeclarationFinish, DeclarationFinished, DeclarationReceipt,
        DeclarationWork, Session, entries,
        session::{GenerationReceipt, GenerationStart, GenerationStarted, GenerationWork},
    },
    project::{
        PreparedRefresh, RefreshRequest,
        settings::{SettingsPublication, SettingsWork, SettingsWritten},
    },
};
use std::{path::PathBuf, sync::Arc};
use tokio::sync::mpsc::UnboundedSender;

#[derive(Default)]
pub struct Tools {
    pub sender: Option<UnboundedSender<Job>>,
    pending: Option<Arc<Request>>,
}
pub struct Request {
    root: PathBuf,
    kind: ToolKind,
}
#[derive(PartialEq, Eq)]
enum ToolKind {
    Refresh,
    Settings,
    Other,
    Catalogue,
}
pub struct Job {
    pub request: Arc<Request>,
    pub work: Work,
}
pub enum Work {
    Start(Box<GenerationStart>),
    Publish(Box<GenerationWork>),
    Refresh(Box<RefreshRequest>),
    Declaration(Box<DeclarationWork>),
    Settings(Box<SettingsWork>),
    SettingsPublish(Box<SettingsPublication>),
    Project(Box<project::Work>),
    Catalogue(Box<crate::localisation::Work>),
}
pub enum Value {
    Started(GenerationStarted),
    Published {
        receipt: Box<GenerationReceipt>,
        rows: Vec<Row>,
    },
    Refreshed(Box<PreparedRefresh>),
    Declaration(Box<declaration::Prepared>),
    DeclarationFinished {
        receipt: DeclarationFinished,
        completion: declaration::Completion,
    },
    Settings(Box<settings::Prepared>),
    SettingsWritten(Box<SettingsWritten>),
    Project(Box<project::Completed>),
    Catalogue(Box<crate::localisation::Completed>),
    CatalogueFinished(Box<crate::localisation::Finished>),
}
pub struct Completed {
    pub request: Arc<Request>,
    pub result: Result<Value, String>,
}
pub enum Retirement {
    FinishCatalogue {
        work: Box<crate::localisation::Finish>,
        retired: Option<Value>,
    },
    Drop(Option<Value>),
    Finish {
        work: Box<DeclarationFinish>,
        completion: declaration::Completion,
        retired: Option<Value>,
    },
}
impl Retirement {
    pub fn finish(self, request: Arc<Request>) -> Option<Completed> {
        match self {
            Self::FinishCatalogue { work, retired } => {
                drop(retired);
                Some(Completed {
                    request,
                    result: Ok(Value::CatalogueFinished(Box::new(work.execute()))),
                })
            }
            Self::Drop(value) => {
                drop(value);
                None
            }
            Self::Finish {
                work,
                completion,
                retired,
            } => {
                drop(retired);
                Some(Completed {
                    request,
                    result: Ok(Value::DeclarationFinished {
                        receipt: work.execute(),
                        completion,
                    }),
                })
            }
        }
    }
}
impl Tools {
    pub fn busy(&self) -> bool {
        self.pending.is_some()
    }
}
impl Writer {
    pub(crate) fn submit_catalogue_tool(&mut self, work: crate::localisation::Work) {
        self.submit_tool(Work::Catalogue(Box::new(work)));
    }
    pub fn declaration_session(&self) -> Option<&Session> {
        self.session.project()?.files().declarations.as_ref()
    }
    pub fn declaration_session_mut(&mut self) -> Option<&mut Session> {
        self.session
            .project_mut()?
            .parts_mut()
            .0
            .declarations
            .as_mut()
    }
    pub fn declaration_action_blocked(&self) -> bool {
        self.project_action_blocked()
            || self
                .session
                .project()
                .is_some_and(|project| project.files().require_navigation_ready().is_err())
    }
    pub fn project_action_blocked(&self) -> bool {
        self.session.read_only()
            || self.phase != Phase::Writing
            || self.work.busy()
            || self.navigation.busy()
            || self.project_jobs_busy()
    }
    pub fn regenerate_schema(&mut self) {
        if self.declaration_action_blocked() {
            return;
        }
        self.workspaces.schema_feedback.clear();
        self.workspaces.schema_feedback_page = 0;
        let result = self
            .declaration_session_mut()
            .ok_or("Open declarations first.".to_owned())
            .and_then(Session::prepare_generation);
        match result {
            Ok(work) => self.submit_tool(Work::Start(Box::new(work))),
            Err(error) => self.schema_feedback(error),
        }
    }
    pub fn cancel_schema_generation(&mut self) {
        if let Some(session) = self.declaration_session() {
            session.cancel();
            self.status = "Cancelling generation; waiting for its terminal result.".into();
        }
    }
    pub(super) fn poll_schema_generation(&mut self) {
        if self.workspaces.tools.busy() || self.work.busy() {
            return;
        }
        let result = self
            .declaration_session_mut()
            .and_then(Session::poll_generation);
        match result {
            Some(Ok(work)) => self.submit_tool(Work::Publish(Box::new(work))),
            Some(Err(error)) => self.schema_feedback(error),
            None => (),
        }
    }
    pub(crate) fn schema_feedback(&mut self, message: String) {
        self.status = message
            .lines()
            .next()
            .unwrap_or("Schema operation finished")
            .chars()
            .take(240)
            .collect();
        self.workspaces.schema_feedback = super::text::pages(&message);
        self.workspaces.schema_feedback_page = 0;
    }
    pub fn refresh_project_context(&mut self) {
        if self.phase != Phase::Writing || self.workspaces.tools.busy() || self.work.busy() {
            return;
        }
        let result = self
            .session
            .project()
            .ok_or("Open a project first.".to_owned())
            .and_then(|project| {
                project
                    .files()
                    .refresh_request(project.current().model())
                    .map_err(|error| error.to_string())
            });
        match result {
            Ok(work) => self.submit_tool(Work::Refresh(Box::new(work))),
            Err(error) => {
                self.status = format!(
                    "{}; context refresh failed: {error}",
                    self.workspaces
                        .context_refresh
                        .map(super::ContextUpdate::message)
                        .unwrap_or("Project settings changed")
                )
            }
        }
    }
    fn submit_tool(&mut self, work: Work) {
        let Some(root) = self.session.project_root() else {
            return;
        };
        let request = Arc::new(Request {
            root: root.to_owned(),
            kind: match &work {
                Work::Refresh(_) => ToolKind::Refresh,
                Work::Settings(_) | Work::SettingsPublish(_) => ToolKind::Settings,
                Work::Catalogue(_) => ToolKind::Catalogue,
                _ => ToolKind::Other,
            },
        });
        self.workspaces.tools.pending = Some(request.clone());
        self.status = match &work {
            Work::Start(_) => "Checking the reviewed producer command…",
            Work::Publish(_) => "Publishing generated schema… You can keep writing.",
            Work::Refresh(_) => "Refreshing project analysis… You can keep writing.",
            Work::Declaration(_) => "Preparing declaration source… Your current draft stays open.",
            Work::Settings(_) => "Preparing project settings… Your manuscript stays open.",
            Work::SettingsPublish(_) => {
                "Publishing reviewed project settings… You can keep writing."
            }
            Work::Project(_) => {
                "Preparing the project operation… Editing is paused until its result returns."
            }
            Work::Catalogue(_) => "Preparing the catalogue… Your source manuscript remains open.",
        }
        .into();
        let job = Job { request, work };
        let sent = match &self.workspaces.tools.sender {
            Some(sender) => sender.send(job).map_err(|error| error.0),
            None => Err(job),
        };
        if let Err(job) = sent {
            let retired = self.accept_tool(
                job.not_started("The project-tools worker stopped. Reopen Writer to retry.".into()),
            );
            if let Some(runtime) = &self.navigation.runtime {
                runtime.spawn_blocking(move || drop(retired));
            }
        }
    }
}
#[cfg(any(test, feature = "test-support"))]
pub mod tests;
