use super::*;
use recite_writer_model::{EditRequest, PreparedEdit};
use recite_writer_session::{
    navigation::{Prepare, Prepared},
    project::{Checkpoint, ReadyCheckpoint},
};
use std::sync::mpsc;
use tokio::sync::mpsc::UnboundedReceiver;
use xilem::{
    ViewCtx,
    core::{MessageProxy, NoElement, View},
};

pub(crate) struct Job {
    pub request: Arc<Request>,
    pub work: Work,
}
pub(crate) enum Work {
    Repair(Box<repair::Work>),
    Edit {
        edit: Box<EditRequest>,
        operation: EditOperation,
    },
    Navigate {
        prepare: Box<Prepare>,
        checkpoint: Option<Box<Checkpoint>>,
    },
}
pub(crate) enum ResultValue {
    Repair(Box<repair::ResultValue>),
    Edit {
        prepared: Box<PreparedEdit>,
        operation: EditOperation,
    },
    Navigation {
        prepared: Box<Prepared>,
        checkpoint: Option<ReadyCheckpoint>,
    },
}
pub(crate) struct Completed {
    pub request: Arc<Request>,
    pub result: Result<ResultValue, String>,
}
enum Output {
    Ready(tokio::runtime::Handle),
    Prepared {
        completed: Box<Completed>,
        retire: mpsc::SyncSender<Option<ResultValue>>,
    },
    Retired(Arc<Request>),
}
impl std::fmt::Debug for Output {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("NavigationOutput").finish_non_exhaustive()
    }
}
impl Job {
    pub fn retire(self) {
        if let Work::Navigate { checkpoint, .. } = self.work {
            let _ = checkpoint.map(|checkpoint| checkpoint.wait());
        }
    }
    pub fn execute(self) -> Completed {
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.work.execute(self.request.as_ref())
        }))
        .unwrap_or_else(|_| {
            Err("Navigation stopped unexpectedly. Your current work stays open.".into())
        });
        Completed {
            request: self.request,
            result,
        }
    }
}
impl Work {
    fn execute(self, control: &dyn WorkControl) -> Result<ResultValue, String> {
        match self {
            Self::Repair(work) => work.execute(control).map(ResultValue::Repair),
            Self::Edit { edit, operation } => edit
                .prepare(control)
                .map(|prepared| ResultValue::Edit {
                    prepared: Box::new(prepared),
                    operation,
                })
                .map_err(|error| feedback::workbench(&error)),
            Self::Navigate {
                prepare,
                checkpoint,
            } => {
                let prepared = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
                    prepare.execute(control)
                }))
                .unwrap_or_else(|_| {
                    Err(
                        recite_writer_session::navigation::NavigationError::Location(
                            "Navigation stopped unexpectedly.".into(),
                        ),
                    )
                });
                let prepared = prepared.map_err(|error| match error {
                    recite_writer_session::navigation::NavigationError::Selection(error) => {
                        feedback::workbench(&error)
                    }
                    recite_writer_session::navigation::NavigationError::File(error) => {
                        feedback::file(&error)
                    }
                    recite_writer_session::navigation::NavigationError::Location(message) => {
                        message
                    }
                });
                // Always join transferred parking, even if preparation failed or was cancelled.
                let checkpoint = checkpoint
                    .map(|checkpoint| checkpoint.wait())
                    .transpose()
                    .map_err(|error| error.to_string());
                Ok(ResultValue::Navigation {
                    prepared: Box::new(prepared?),
                    checkpoint: checkpoint?,
                })
            }
        }
    }
}
pub(crate) fn worker() -> impl View<Writer, (), ViewCtx, Element = NoElement> {
    xilem::view::worker(
        serve,
        |writer: &mut Writer, sender| writer.navigation.sender = Some(sender),
        |writer: &mut Writer, output: Output| match output {
            Output::Ready(runtime) => writer.navigation.runtime = Some(runtime),
            Output::Prepared { completed, retire } => {
                let retired = writer.accept_navigation(*completed);
                let _ = retire.send(retired);
            }
            Output::Retired(request) => writer.navigation_retired(&request),
        },
    )
}
async fn serve(proxy: MessageProxy<Output>, mut requests: UnboundedReceiver<Job>) {
    if proxy
        .message(Output::Ready(tokio::runtime::Handle::current()))
        .is_err()
    {
        return;
    }
    while let Some(job) = requests.recv().await {
        let proxy = proxy.clone();
        let _ = tokio::task::spawn_blocking(move || {
            let completed = Box::new(job.execute());
            let request = completed.request.clone();
            let (retire, retired) = mpsc::sync_channel(1);
            if proxy
                .message(Output::Prepared { completed, retire })
                .is_ok()
            {
                drop(retired.recv());
                let _ = proxy.message(Output::Retired(request));
            }
        })
        .await;
    }
}
