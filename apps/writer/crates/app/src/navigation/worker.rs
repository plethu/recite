use super::*;
use recite_writer_model::{EditRequest, PreparedEdit};
use recite_writer_session::{
    navigation::{Prepare, Prepared},
    project::{Checkpoint, PreparedReload, ReadyCheckpoint, ReloadRequest},
};

pub struct Job {
    pub request: Arc<Request>,
    pub work: Work,
}
pub enum Work {
    Present(Box<destination::Work>),
    ProjectWorkspace(crate::workspaces::Work),
    Reload(Box<ReloadRequest>),
    Map(Box<crate::map::Preparation>),
    Repair(Box<repair::Work>),
    Structure(Box<recite_writer_model::StructuralRequest>),
    Edit {
        edit: Box<EditRequest>,
        operation: EditOperation,
    },
    Navigate {
        prepare: Box<Prepare>,
        checkpoint: Option<Box<Checkpoint>>,
    },
}
pub enum ResultValue {
    Presented(Box<destination::Prepared>),
    ProjectWorkspace(crate::workspaces::Prepared),
    Reload(Box<PreparedReload>),
    Map(Box<crate::map::Prepared>),
    Repair(Box<repair::ResultValue>),
    Structure(Box<recite_writer_model::PreparedStructuralEdit>),
    Edit {
        prepared: Box<PreparedEdit>,
        operation: EditOperation,
    },
    Navigation {
        prepared: Box<Prepared>,
        checkpoint: Option<ReadyCheckpoint>,
    },
}
pub struct Completed {
    pub request: Arc<Request>,
    pub result: Result<ResultValue, String>,
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
    pub fn execute(self, control: &dyn WorkControl) -> Result<ResultValue, String> {
        match self {
            Self::Present(work) => Ok(ResultValue::Presented(Box::new(
                match control.checkpoint() {
                    Ok(()) => work.execute(control),
                    Err(error) => work.not_started(error.to_string()),
                },
            ))),
            Self::ProjectWorkspace(work) => {
                control.checkpoint().map_err(|error| error.to_string())?;
                work.execute().map(ResultValue::ProjectWorkspace)
            }
            Self::Reload(reload) => reload
                .prepare(control)
                .map(|prepared| ResultValue::Reload(Box::new(prepared)))
                .map_err(|error| feedback::file(&error)),
            Self::Map(work) => work
                .execute(control)
                .map(|prepared| ResultValue::Map(Box::new(prepared))),
            Self::Repair(work) => work.execute(control).map(ResultValue::Repair),
            Self::Structure(edit) => edit
                .prepare(control)
                .map(|prepared| ResultValue::Structure(Box::new(prepared)))
                .map_err(|error| feedback::workbench(&error)),
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
impl ResultValue {
    /// Displaced models and recovery fences are retired by the preparation worker.
    pub fn retire(mut self) -> Result<(), String> {
        if let Self::Reload(prepared) = &mut self {
            prepared.finish_cleanup().map_err(|error| {
                format!(
                    "Reload accepted for {}; recovery cleanup failed: {}. The recovery copy is preserved.",
                    prepared.document().display(),
                    feedback::file(&error)
                )
            })?;
        }
        Ok(())
    }
}
