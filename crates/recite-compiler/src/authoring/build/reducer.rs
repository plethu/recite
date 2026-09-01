use super::super::candidates_are_ordered;
use super::super::identity::BuildGeneration;
use super::super::result::BuildTerminalStatus;
use super::phase::BuildEventKind;
use super::state::{BuildState, BuildTransition, BuildTransitionError};

#[path = "reducer_support.rs"]
mod support;
use support::invalid;
#[path = "reducer_terminal.rs"]
mod terminal;

/// Pure reducer for legal build lifecycle transitions.
#[non_exhaustive]
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct BuildLifecycle {
    state: BuildState,
    latest_generation: Option<BuildGeneration>,
}
impl BuildLifecycle {
    #[must_use]
    pub const fn new() -> Self {
        Self {
            state: BuildState::Idle,
            latest_generation: None,
        }
    }
    #[must_use]
    pub const fn state(&self) -> &BuildState {
        &self.state
    }
    pub fn transition(
        &mut self,
        event: BuildTransition,
    ) -> Result<&BuildState, BuildTransitionError> {
        let next = self.next_state(&event)?;
        if let BuildTransition::Start { request } = &event {
            self.latest_generation = Some(request.generation());
        }
        self.state = next;
        Ok(&self.state)
    }
    fn next_state(&self, event: &BuildTransition) -> Result<BuildState, BuildTransitionError> {
        match event {
            BuildTransition::Start { request } => {
                if !self.state.is_terminal() && !matches!(self.state, BuildState::Idle) {
                    return Err(invalid(&self.state, BuildEventKind::Start));
                }
                if let Some(previous) = self.latest_generation
                    && request.generation() <= previous
                {
                    return Err(BuildTransitionError::GenerationNotNewer {
                        previous,
                        received: request.generation(),
                    });
                }
                Ok(BuildState::Checking {
                    request: request.clone(),
                })
            }
            BuildTransition::CheckPassed {
                freshness,
                diagnostics,
            } => {
                if diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic.severity == recite_core::DiagnosticSeverity::Error)
                {
                    return Err(BuildTransitionError::CheckContainsErrors);
                }
                match &self.state {
                    BuildState::Checking { request }
                        if freshness.expected() == request.fingerprints() =>
                    {
                        Ok(BuildState::Building {
                            request: request.clone(),
                            candidates: Vec::new(),
                            diagnostics: diagnostics.clone(),
                            freshness: freshness.clone(),
                        })
                    }
                    BuildState::Checking { .. } => Err(BuildTransitionError::FreshnessMismatch),
                    _ => Err(invalid(&self.state, BuildEventKind::CheckPassed)),
                }
            }
            BuildTransition::BuildCompleted { candidates } => match &self.state {
                BuildState::Building {
                    request,
                    diagnostics,
                    freshness,
                    ..
                } => {
                    if !candidates_are_ordered(candidates) {
                        return Err(BuildTransitionError::CandidatesOutOfOrder);
                    }
                    Ok(BuildState::Ready {
                        request: request.clone(),
                        candidates: candidates.clone(),
                        diagnostics: diagnostics.clone(),
                        freshness: freshness.clone(),
                    })
                }
                _ => Err(invalid(&self.state, BuildEventKind::BuildCompleted)),
            },
            BuildTransition::PublishStarted { prepared } => match &self.state {
                BuildState::Ready {
                    request,
                    candidates,
                    diagnostics,
                    freshness,
                } if prepared.request_identity()
                    == &super::super::request_identity::BuildRequestIdentity::from_request(
                        request,
                    )
                    && prepared.candidates() == candidates =>
                {
                    Ok(BuildState::Publishing {
                        request: request.clone(),
                        prepared: prepared.clone(),
                        diagnostics: diagnostics.clone(),
                        freshness: freshness.clone(),
                    })
                }
                BuildState::Ready { .. } => Err(BuildTransitionError::PreparedIdentityMismatch),
                _ => Err(invalid(&self.state, BuildEventKind::PublishStarted)),
            },
            BuildTransition::CheckFailed { result } => self.terminal(
                BuildEventKind::CheckFailed,
                BuildTerminalStatus::Failed,
                result,
                true,
            ),
            BuildTransition::NoCandidates { result } => self.terminal(
                BuildEventKind::NoCandidates,
                BuildTerminalStatus::Succeeded,
                result,
                true,
            ),
            BuildTransition::PublishCompleted { result } => self.terminal(
                BuildEventKind::PublishCompleted,
                BuildTerminalStatus::Succeeded,
                result,
                true,
            ),
            BuildTransition::Cancelled { result } => self.terminal(
                BuildEventKind::Cancelled,
                BuildTerminalStatus::Cancelled,
                result,
                false,
            ),
            BuildTransition::Superseded { result } => self.terminal(
                BuildEventKind::Superseded,
                BuildTerminalStatus::Superseded,
                result,
                false,
            ),
            BuildTransition::Stale { result } => self.terminal(
                BuildEventKind::Stale,
                BuildTerminalStatus::Stale,
                result,
                false,
            ),
            BuildTransition::Failed { result } => self.terminal(
                BuildEventKind::Failed,
                BuildTerminalStatus::Failed,
                result,
                false,
            ),
            BuildTransition::FreshnessFinalized { finalization } => {
                self.finalize_freshness(finalization)
            }
        }
    }

    fn finalize_freshness(
        &self,
        finalization: &super::super::freshness::FreshnessFinalization,
    ) -> Result<BuildState, BuildTransitionError> {
        let BuildState::Succeeded { result } = &self.state else {
            return Err(invalid(&self.state, BuildEventKind::FreshnessFinalized));
        };
        if !matches!(
            result.publish(),
            super::super::publish::PublishOutcome::Published { .. }
        ) {
            return Err(BuildTransitionError::FreshnessFinalizationPublishMismatch);
        }
        let assessment = match finalization {
            super::super::freshness::FreshnessFinalization::Fresh { assessment, .. }
                if assessment.status() == super::super::freshness::FreshnessStatus::Fresh =>
            {
                assessment
            }
            super::super::freshness::FreshnessFinalization::Stale { assessment, .. }
                if assessment.status() == super::super::freshness::FreshnessStatus::Stale =>
            {
                assessment
            }
            super::super::freshness::FreshnessFinalization::Indeterminate {
                assessment, ..
            } if assessment.status() == super::super::freshness::FreshnessStatus::Unknown => {
                assessment
            }
            _ => return Err(BuildTransitionError::FreshnessFinalizationAssessmentMismatch),
        };
        if assessment.expected() != result.fingerprints() {
            return Err(BuildTransitionError::FreshnessMismatch);
        }
        let mut result = result.clone();
        result.finalize_freshness(finalization.clone());
        Ok(match result.status() {
            BuildTerminalStatus::Succeeded => BuildState::Succeeded { result },
            BuildTerminalStatus::Stale => BuildState::Stale { result },
            BuildTerminalStatus::Failed => BuildState::Failed { result },
            status => {
                return Err(BuildTransitionError::ResultStatusMismatch {
                    expected: BuildTerminalStatus::Failed,
                    status,
                });
            }
        })
    }
}
