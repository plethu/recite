use super::super::identity::BuildGeneration;
use super::super::result::BuildTerminalStatus;
use super::phase::BuildEventKind;
use super::state::{BuildState, BuildTransition, BuildTransitionError};

#[path = "reducer_support.rs"]
mod support;
use support::invalid;

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
            } => match &self.state {
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
            },
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
        }
    }
    fn terminal(
        &self,
        event: BuildEventKind,
        expected: BuildTerminalStatus,
        result: &super::super::result::BuildResult,
        strict_phase: bool,
    ) -> Result<BuildState, BuildTransitionError> {
        let (request, candidates) = match &self.state {
            BuildState::Checking { request } => (request, Vec::new()),
            BuildState::Building {
                request,
                candidates,
                ..
            }
            | BuildState::Ready {
                request,
                candidates,
                ..
            } => (request, candidates.clone()),
            BuildState::Publishing {
                request, prepared, ..
            } => (request, prepared.candidates().to_vec()),
            _ => return Err(invalid(&self.state, event)),
        };
        if strict_phase
            && event == BuildEventKind::CheckFailed
            && !matches!(self.state, BuildState::Checking { .. })
        {
            return Err(invalid(&self.state, event));
        }
        if strict_phase
            && event == BuildEventKind::NoCandidates
            && !matches!(self.state, BuildState::Ready { .. })
        {
            return Err(invalid(&self.state, event));
        }
        if !result.matches_request(request) {
            return Err(BuildTransitionError::ResultIdentityMismatch);
        }
        if result.candidates() != candidates {
            return Err(BuildTransitionError::ResultCandidatesMismatch);
        }
        if event == BuildEventKind::PublishCompleted
            && !matches!(self.state, BuildState::Publishing { .. })
        {
            return Err(invalid(&self.state, event));
        }
        if event == BuildEventKind::PublishCompleted
            && !matches!(
                result.publish(),
                super::super::publish::PublishOutcome::Published { .. }
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
fn candidates_are_ordered(candidates: &[super::super::publish::BuildCandidate]) -> bool {
    candidates
        .windows(2)
        .all(|pair| pair[0].target() <= pair[1].target())
}
