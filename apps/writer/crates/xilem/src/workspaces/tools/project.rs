//! Exclusive project actions always return the real owner, including failed dispatch and unwind.
use super::*;
use crate::{
    session::{Checkout, RecoveryReason},
    workspaces::{rename, text},
};
use recite_compiler::authoring::WorkControl;
use recite_writer_session::{FileError, ProjectSession, comparison::ComparisonRow};
#[cfg(test)]
mod tests;

pub(crate) enum Action {
    ReviewRename { block: String, name: String },
    ApplyRename,
    History { redo: bool },
    FinishRecovery,
}
pub(in crate::workspaces) struct Work {
    checkout: Checkout,
    action: Action,
    was_recovering: Option<RecoveryReason>,
}
pub(super) struct Completed {
    checkout: Checkout,
    outcome: Outcome,
    pages: Vec<rename::Change>,
}
enum Outcome {
    Ready(Result<&'static str, String>),
    RecoveryRequired {
        reason: RecoveryReason,
        message: String,
    },
}
impl Work {
    pub(super) fn not_started(self, error: String) -> Completed {
        let outcome = match self.was_recovering {
            Some(reason) => Outcome::RecoveryRequired {
                reason,
                message: error,
            },
            None => Outcome::Ready(Err(error)),
        };
        Completed {
            checkout: self.checkout,
            outcome,
            pages: vec![],
        }
    }
    pub(super) fn execute(self) -> Completed {
        self.execute_with(Action::perform)
    }
    fn execute_with(
        mut self,
        perform: impl FnOnce(
            &Action,
            &mut ProjectSession,
            &dyn WorkControl,
        ) -> Result<&'static str, FileError>,
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
                .map(|project| describe(project))
                .unwrap_or_default()
        }));
        let pages = match pages {
            Ok(pages) => pages,
            Err(_) => {
                if let Some(project) = self.checkout.project.as_mut() {
                    project.parts_mut().0.rename_review = None;
                }
                if matches!(outcome, Outcome::Ready(_)) {
                    outcome=Outcome::Ready(Err("The project returned, but its review could not be rendered. Review again before applying.".into()));
                }
                vec![]
            }
        };
        Completed {
            checkout: self.checkout,
            outcome,
            pages,
        }
    }
}
impl Action {
    fn perform(
        &self,
        project: &mut ProjectSession,
        control: &dyn WorkControl,
    ) -> Result<&'static str, FileError> {
        let (files, model) = project.parts_mut();
        match self {
            Self::ReviewRename { block, name } => {
                control.checkpoint()?;
                files.review_rename(model, block, name)?;
                control.checkpoint()?;
                Ok("Rename reviewed. Inspect every change, then Apply project rename.")
            }
            Self::ApplyRename => {
                files.apply_rename_with_control(model, control)?;
                Ok("Project rename staged. Save all to write the project files.")
            }
            Self::History { redo } => {
                if files.rename_history_with_control(model, *redo, control)? {
                    Ok("Grouped project history restored. Save all to write the project files.")
                } else {
                    Ok("No grouped project change is available for this document.")
                }
            }
            Self::FinishRecovery => {
                files.finish_project_edit(model)?;
                Ok("Project recovery completed. Editing is available.")
            }
        }
    }
}
fn describe(project: &ProjectSession) -> Vec<rename::Change> {
    let Some(review) = &project.files().rename_review else {
        return vec![];
    };
    let rows = review
        .plan
        .changes
        .iter()
        .flat_map(|change| {
            ComparisonRow::source(change.document.as_str(), &change.before, &change.after)
        })
        .chain(
            (review.manifest_before != review.manifest_after)
                .then(|| {
                    ComparisonRow::source(
                        "recite.project.toml",
                        &review.manifest_before,
                        &review.manifest_after,
                    )
                })
                .into_iter()
                .flatten(),
        );
    rows.filter(|row| row.before != row.after)
        .flat_map(|row| {
            let before = row.before.as_deref().map(text::pages).unwrap_or_default();
            let after = row.after.as_deref().map(text::pages).unwrap_or_default();
            let count = before.len().max(after.len());
            (0..count).map(move |index| rename::Change {
                caption: format!("{} · part {} of {count}", row.caption, index + 1),
                before: before.get(index).cloned(),
                after: after.get(index).cloned(),
            })
        })
        .collect()
}
impl Writer {
    pub(crate) fn project_action(&mut self, action: Action) {
        let retry = matches!(action, Action::FinishRecovery);
        if self.phase != Phase::Writing
            || self.navigation.busy()
            || self.work.busy()
            || self.project_jobs_busy()
            || self.trial.busy()
            || (self.session.read_only() && !retry)
        {
            return;
        }
        if retry && self.session.recovery_reason() != Some(RecoveryReason::RetryCheckpoint) {
            self.status = FileError::ProjectEditRecoveryRestart.to_string();
            return;
        }
        let was_recovering = self.session.recovery_reason();
        match self.session.checkout() {
            Ok(checkout) => self.submit_tool(super::Work::Project(Box::new(Work {
                checkout,
                action,
                was_recovering,
            }))),
            Err(error) => self.status = error.to_string(),
        }
    }
    pub(super) fn accept_project_tool(&mut self, completed: &mut Completed) -> Result<(), String> {
        self.session
            .restore_checkout(
                &mut completed.checkout,
                match &completed.outcome {
                    Outcome::RecoveryRequired { reason, .. } => Some(*reason),
                    _ => None,
                },
            )
            .map_err(|error| error.to_string())?;
        self.workspaces
            .rename
            .replace(std::mem::take(&mut completed.pages));
        self.reading.prepare(self.session.model());
        self.refresh_rules();
        self.invalidate_map();
        self.refresh_map_if_needed();
        let details = match &completed.outcome {
            Outcome::RecoveryRequired {
                reason: RecoveryReason::ReopenRequired,
                ..
            }
            | Outcome::Ready(Ok(_)) => None,
            Outcome::Ready(Err(error)) | Outcome::RecoveryRequired { message: error, .. } => {
                Some(error.as_str())
            }
        };
        self.workspaces.rename.report(details);
        let message = match &completed.outcome {
            Outcome::Ready(Ok(message)) => *message,
            Outcome::Ready(Err(error)) => error.as_str(),
            Outcome::RecoveryRequired {
                reason: RecoveryReason::RetryCheckpoint,
                ..
            } => "Grouped project recovery is incomplete.",
            Outcome::RecoveryRequired {
                reason: RecoveryReason::ReopenRequired,
                ..
            } => "Project operation stopped unexpectedly.",
        };
        self.status = message
            .lines()
            .next()
            .unwrap_or("Project operation finished")
            .chars()
            .take(240)
            .collect();
        Ok(())
    }
    pub(crate) fn cancel_project_action(&mut self) {
        self.session.cancel_exclusive();
        self.status =
            "Cancellation requested; waiting for the project owner and recovery outcome.".into();
    }
}
