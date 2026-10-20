use super::*;
use crate::workspaces::tools::Work as ToolWork;
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
            Ok(checkout) => self.submit_tool(ToolWork::Project(Box::new(Work {
                checkout,
                action,
                was_recovering,
            }))),
            Err(error) => self.status = error.to_string(),
        }
    }
    pub(in crate::workspaces::tools) fn accept_project_tool(
        &mut self,
        completed: &mut Completed,
    ) -> Result<(), String> {
        self.session
            .restore_checkout(
                &mut completed.checkout,
                match &completed.outcome {
                    Outcome::RecoveryRequired { reason, .. } => Some(*reason),
                    _ => None,
                },
            )
            .map_err(|error| error.to_string())?;
        match completed.presentation.take() {
            Some(Presentation::Rename(pages)) => self.workspaces.rename.replace(pages),
            Some(Presentation::External(inspection)) => {
                if let Some(archive) = inspection
                    .as_ref()
                    .and_then(|inspection| inspection.archive.as_ref())
                {
                    self.recovery_copy = Some(archive.clone());
                }
                self.workspaces.external.replace(inspection)
            }
            None => (),
        }
        if let Outcome::Ready(Ok(success)) = &completed.outcome
            && let Some(archive) = &success.archive
        {
            self.recovery_copy = Some(archive.clone());
        }
        self.reading.prepare(self.session.model());
        self.refresh_rules();
        self.invalidate_map();
        self.refresh_map_if_needed();
        let details = match &completed.outcome {
            Outcome::RecoveryRequired {
                reason: RecoveryReason::ReopenRequired,
                ..
            } => None,
            Outcome::Ready(Ok(success)) => success.warning.as_deref(),
            Outcome::Ready(Err(error)) | Outcome::RecoveryRequired { message: error, .. } => {
                Some(error.as_str())
            }
        };
        match completed.family {
            Family::Rename => self.workspaces.rename.report(details),
            Family::External => self.workspaces.external.report(details),
        }
        let message = match &completed.outcome {
            Outcome::Ready(Ok(success)) if success.warning.is_some() => {
                "Document adopted; recovery is incomplete. Retry Save or keep this window open."
            }
            Outcome::Ready(Ok(success)) => success.message,
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
