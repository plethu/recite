//! One project-tool operation owns its terminal receipt across screen changes.
mod accepting;
mod job;
mod source;
mod worker;
use super::{Phase, Writer, declarations::Row};
use recite_writer_session::{
    declarations::{
        Session, SourceAction, SourceReceipt, SourceRecovered, SourceRecovery, SourceWork, entries,
        session::{GenerationReceipt, GenerationStart, GenerationStarted, GenerationWork},
    },
    project::{PreparedRefresh, RefreshRequest},
};
use std::{path::PathBuf, sync::Arc};
use tokio::sync::mpsc::UnboundedSender;
pub(crate) use worker::worker;

#[derive(Default)]
pub(super) struct Tools {
    pub sender: Option<UnboundedSender<Job>>,
    pending: Option<Arc<Request>>,
}
struct Request {
    root: PathBuf,
    kind: ToolKind,
}
#[derive(PartialEq, Eq)]
enum ToolKind {
    Refresh,
    Other,
}
pub(super) struct Job {
    request: Arc<Request>,
    work: Work,
}
pub(super) enum Work {
    Start(Box<GenerationStart>),
    Publish(Box<GenerationWork>),
    Refresh(Box<RefreshRequest>),
    Source(Box<SourceWork>),
}
enum Value {
    Started(GenerationStarted),
    Published {
        receipt: Box<GenerationReceipt>,
        rows: Vec<Row>,
    },
    Refreshed(Box<PreparedRefresh>),
    Source(Box<source::Prepared>),
    SourceRecovered(SourceRecovered),
}
struct Completed {
    request: Arc<Request>,
    result: Result<Value, String>,
}
enum Retirement {
    Drop(Option<Value>),
    Recover(Box<SourceRecovery>, Option<Value>),
}
impl Retirement {
    fn finish(self, request: Arc<Request>) -> Option<Completed> {
        match self {
            Self::Drop(value) => {
                drop(value);
                None
            }
            Self::Recover(work, value) => {
                drop(value);
                Some(Completed {
                    request,
                    result: Ok(Value::SourceRecovered(work.execute())),
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
    pub(crate) fn declaration_session(&self) -> Option<&Session> {
        self.session.project()?.files().declarations.as_ref()
    }
    fn declaration_session_mut(&mut self) -> Option<&mut Session> {
        self.session
            .project_mut()?
            .parts_mut()
            .0
            .declarations
            .as_mut()
    }
    pub(super) fn declaration_action_blocked(&self) -> bool {
        self.phase != Phase::Writing
            || self.work.busy()
            || self.navigation.busy()
            || self.project_jobs_busy()
    }
    pub(crate) fn regenerate_schema(&mut self) {
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
    pub(crate) fn cancel_schema_generation(&mut self) {
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
    pub(crate) fn refresh_schema_context(&mut self) {
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
                self.status = format!("Schema published; context refresh failed: {error}")
            }
        }
    }
    fn submit_tool(&mut self, work: Work) {
        let Some(project) = self.session.project() else {
            return;
        };
        let request = Arc::new(Request {
            root: project.root().to_owned(),
            kind: if matches!(&work, Work::Refresh(_)) {
                ToolKind::Refresh
            } else {
                ToolKind::Other
            },
        });
        self.workspaces.tools.pending = Some(request.clone());
        self.status = match &work {
            Work::Start(_) => "Checking the reviewed producer command…",
            Work::Publish(_) => "Publishing generated schema… You can keep writing.",
            Work::Refresh(_) => "Refreshing schema analysis… You can keep writing.",
            Work::Source(_) => "Preparing declaration source… Your current draft stays open.",
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
#[cfg(test)]
mod tests;
