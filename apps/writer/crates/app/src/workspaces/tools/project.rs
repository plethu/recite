//! Exclusive project actions always return the real owner, including failed dispatch and unwind.
use super::*;
use crate::{
    session::{Checkout, RecoveryReason},
    workspaces::{
        external::{Inspection, Resolved},
        rename, text,
    },
};
use recite_compiler::authoring::WorkControl;
use recite_writer_session::{FileError, ProjectSession};
use std::path::PathBuf;
pub mod accepting;
pub mod external;
pub mod renaming;
pub use external::Action as ExternalAction;
#[cfg(any(test, feature = "test-support"))]
pub mod tests;

pub enum Action {
    ReviewRename { block: String, name: String },
    ApplyRename,
    History { redo: bool },
    FinishRecovery,
    External(external::Action),
}
pub struct Work {
    checkout: Checkout,
    action: Action,
    was_recovering: Option<RecoveryReason>,
}
pub struct Completed {
    checkout: Checkout,
    pub outcome: Outcome,
    presentation: Option<Presentation>,
    family: Family,
}
#[derive(Clone, Copy)]
pub enum Family {
    Rename,
    External,
}
impl Action {
    fn family(&self) -> Family {
        match self {
            Self::External(_) => Family::External,
            _ => Family::Rename,
        }
    }
}
pub enum Outcome {
    Ready(Result<Success, String>),
    RecoveryRequired {
        reason: RecoveryReason,
        message: String,
    },
}
impl Work {
    pub fn not_started(self, error: String) -> Completed {
        let outcome = match self.was_recovering {
            Some(reason) => Outcome::RecoveryRequired {
                reason,
                message: error,
            },
            None => Outcome::Ready(Err(error)),
        };
        Completed {
            family: self.action.family(),
            checkout: self.checkout,
            outcome,
            presentation: None,
        }
    }
    pub fn execute(self) -> Completed {
        self.execute_with(Action::perform)
    }
    pub fn execute_with(
        self,
        perform: impl FnOnce(
            &Action,
            &mut ProjectSession,
            &dyn WorkControl,
        ) -> Result<Success, FileError>,
    ) -> Completed {
        self.execute_with_presentation(perform, |action, project| match action {
            Action::External(_) => Presentation::External(external::describe(project)),
            _ => Presentation::Rename(renaming::describe(project)),
        })
    }
    pub fn execute_with_presentation(
        mut self,
        perform: impl FnOnce(
            &Action,
            &mut ProjectSession,
            &dyn WorkControl,
        ) -> Result<Success, FileError>,
        present: impl FnOnce(&Action, &ProjectSession) -> Presentation,
    ) -> Completed {
        let ticket = self.checkout.ticket.clone();
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            let project = self
                .checkout
                .project
                .as_mut()
                .ok_or(FileError::ProjectActionPending)?;
            if !matches!(self.action, Action::FinishRecovery) {
                ticket.checkpoint()?;
                project.checkpoint()?;
            }
            perform(&self.action, project, &ticket)
        }));
        let mut outcome = match result {
            Ok(result) if self.checkout.project.as_ref().is_some_and(|project| {
                project.files().project_edit_recovery_pending()
            }) => {
                let error = result.err().map(|error| error.to_string()).unwrap_or_default();
                Outcome::RecoveryRequired {
                    reason: RecoveryReason::RetryCheckpoint,
                    message: error,
                }
            }
            Ok(result) => Outcome::Ready(result.map_err(|error| error.to_string())),
            Err(_) => Outcome::RecoveryRequired {
                reason: RecoveryReason::ReopenRequired,
                message: "Project operation stopped unexpectedly. Close with recovery and reopen the project before editing.".into(),
            },
        };
        let pages = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            self.checkout
                .project
                .as_ref()
                .map(|project| present(&self.action, project))
        }));
        let pages = match pages {
            Ok(pages) => pages,
            Err(_) => {
                if let Some(project) = self.checkout.project.as_mut() {
                    project.parts_mut().0.rename_review = None;
                }
                let message = "The project returned, but its review could not be rendered. Inspect again before another change.";
                match &mut outcome {
                    Outcome::Ready(Ok(success)) => {
                        success.warning = Some(match success.warning.take() {
                            Some(warning) => format!("{warning}\n{message}"),
                            None => message.into(),
                        });
                    }
                    Outcome::Ready(Err(_)) => outcome = Outcome::Ready(Err(message.into())),
                    Outcome::RecoveryRequired { .. } => (),
                }
                None
            }
        };
        Completed {
            family: self.action.family(),
            checkout: self.checkout,
            outcome,
            presentation: pages,
        }
    }
}

pub struct Success {
    message: &'static str,
    archive: Option<PathBuf>,
    pub warning: Option<String>,
    pub effect: Effect,
}
pub enum Effect {
    None,
    ExternalResolved(Resolved),
}
impl Success {
    fn message(message: &'static str) -> Self {
        Self {
            message,
            archive: None,
            warning: None,
            effect: Effect::None,
        }
    }
}
pub enum Presentation {
    Rename(Vec<rename::Change>),
    External(Option<Inspection>),
}
