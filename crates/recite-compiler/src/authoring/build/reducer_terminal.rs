use super::super::super::result::BuildTerminalStatus;
use super::super::phase::BuildEventKind;
use super::super::state::{BuildState, BuildTransitionError};
use super::BuildLifecycle;

impl BuildLifecycle {
    pub(super) fn terminal(
        &self,
        event: BuildEventKind,
        expected: BuildTerminalStatus,
        result: &super::super::super::result::BuildResult,
        strict_phase: bool,
    ) -> Result<BuildState, BuildTransitionError> {
        let (request, candidates, active_diagnostics, active_freshness) = match &self.state {
            BuildState::Checking { request } => (request, Vec::new(), None, None),
            BuildState::Building {
                request,
                candidates,
                diagnostics,
                freshness,
            }
            | BuildState::Ready {
                request,
                candidates,
                diagnostics,
                freshness,
            } => (
                request,
                candidates.clone(),
                Some(diagnostics),
                Some(freshness),
            ),
            BuildState::Publishing {
                request,
                prepared,
                diagnostics,
                freshness,
            } => (
                request,
                prepared.candidates().to_vec(),
                Some(diagnostics),
                Some(freshness),
            ),
            _ => return Err(super::support::invalid(&self.state, event)),
        };
        if strict_phase
            && event == BuildEventKind::CheckFailed
            && !matches!(self.state, BuildState::Checking { .. })
        {
            return Err(super::support::invalid(&self.state, event));
        }
        if strict_phase
            && event == BuildEventKind::NoCandidates
            && !matches!(self.state, BuildState::Ready { .. })
        {
            return Err(super::support::invalid(&self.state, event));
        }
        if !result.matches_request(request) {
            return Err(BuildTransitionError::ResultIdentityMismatch);
        }
        if result.candidates() != candidates {
            return Err(BuildTransitionError::ResultCandidatesMismatch);
        }
        if let Some(freshness) = active_freshness
            && result.freshness() != freshness
        {
            return Err(BuildTransitionError::ResultFreshnessMismatch);
        }
        if let Some(diagnostics) = active_diagnostics
            && !result.diagnostics().starts_with(diagnostics)
        {
            return Err(BuildTransitionError::ResultDiagnosticsMismatch);
        }
        if event == BuildEventKind::PublishCompleted
            && !matches!(self.state, BuildState::Publishing { .. })
        {
            return Err(super::support::invalid(&self.state, event));
        }
        if event == BuildEventKind::PublishCompleted
            && !matches!(
                result.publish(),
                super::super::super::publish::PublishOutcome::Published { .. }
            )
        {
            return Err(BuildTransitionError::ResultPublishMismatch);
        }
        if result.status() != expected {
            return Err(BuildTransitionError::ResultStatusMismatch {
                expected,
                status: result.status(),
            });
        }
        Ok(match expected {
            BuildTerminalStatus::Succeeded => BuildState::Succeeded {
                result: result.clone(),
            },
            BuildTerminalStatus::Failed => BuildState::Failed {
                result: result.clone(),
            },
            BuildTerminalStatus::Stale => BuildState::Stale {
                result: result.clone(),
            },
            BuildTerminalStatus::Cancelled => BuildState::Cancelled {
                result: result.clone(),
            },
            BuildTerminalStatus::Superseded => BuildState::Superseded {
                result: result.clone(),
            },
        })
    }
}
