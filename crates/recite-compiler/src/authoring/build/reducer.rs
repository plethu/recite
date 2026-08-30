use super::super::identity::BuildGeneration;
use super::super::result::BuildTerminalStatus;
use super::state::{BuildEventKind, BuildPhase, BuildState, BuildTransition, BuildTransitionError};

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
                    generation: request.generation(),
                    snapshot_generation: request.snapshot_generation(),
                })
            }
            BuildTransition::CheckPassed { .. } => match self.state {
                BuildState::Checking {
                    generation,
                    snapshot_generation,
                } => Ok(BuildState::Building {
                    generation,
                    snapshot_generation,
                    candidates: Vec::new(),
                }),
                _ => Err(invalid(&self.state, BuildEventKind::CheckPassed)),
            },
            BuildTransition::BuildCompleted { candidates } => match self.state {
                BuildState::Building {
                    generation,
                    snapshot_generation,
                    ..
                } => Ok(BuildState::Building {
                    generation,
                    snapshot_generation,
                    candidates: candidates.clone(),
                }),
                _ => Err(invalid(&self.state, BuildEventKind::BuildCompleted)),
            },
            BuildTransition::PublishStarted => match self.state {
                BuildState::Building {
                    generation,
                    snapshot_generation,
                    ref candidates,
                } => Ok(BuildState::Publishing {
                    generation,
                    snapshot_generation,
                    candidates: candidates.clone(),
                }),
                _ => Err(invalid(&self.state, BuildEventKind::PublishStarted)),
            },
            BuildTransition::CheckFailed { result } => self.terminal(
                BuildEventKind::CheckFailed,
                BuildTerminalStatus::Failed,
                result,
            ),
            BuildTransition::PublishCompleted { result } => self.terminal(
                BuildEventKind::PublishCompleted,
                BuildTerminalStatus::Succeeded,
                result,
            ),
            BuildTransition::Cancelled { result } => self.terminal(
                BuildEventKind::Cancelled,
                BuildTerminalStatus::Cancelled,
                result,
            ),
            BuildTransition::Superseded { result } => self.terminal(
                BuildEventKind::Superseded,
                BuildTerminalStatus::Superseded,
                result,
            ),
            BuildTransition::Stale { result } => {
                self.terminal(BuildEventKind::Stale, BuildTerminalStatus::Stale, result)
            }
            BuildTransition::Failed { result } => {
                self.terminal(BuildEventKind::Failed, BuildTerminalStatus::Failed, result)
            }
        }
    }
    fn terminal(
        &self,
        event: BuildEventKind,
        expected: BuildTerminalStatus,
        result: &super::super::result::BuildResult,
    ) -> Result<BuildState, BuildTransitionError> {
        let active = self
            .state
            .generation()
            .ok_or_else(|| invalid(&self.state, event))?;
        if !matches!(
            self.state,
            BuildState::Checking { .. }
                | BuildState::Building { .. }
                | BuildState::Publishing { .. }
        ) {
            return Err(invalid(&self.state, event));
        }
        if result.generation() != active {
            return Err(BuildTransitionError::ResultGenerationMismatch {
                active,
                received: result.generation(),
            });
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
fn invalid(state: &BuildState, event: BuildEventKind) -> BuildTransitionError {
    BuildTransitionError::Invalid {
        state: phase(state),
        event,
    }
}
const fn phase(state: &BuildState) -> BuildPhase {
    match state {
        BuildState::Idle => BuildPhase::Idle,
        BuildState::Checking { .. } => BuildPhase::Checking,
        BuildState::Building { .. } => BuildPhase::Building,
        BuildState::Publishing { .. } => BuildPhase::Publishing,
        BuildState::Succeeded { .. } => BuildPhase::Succeeded,
        BuildState::Failed { .. } => BuildPhase::Failed,
        BuildState::Stale { .. } => BuildPhase::Stale,
        BuildState::Cancelled { .. } => BuildPhase::Cancelled,
        BuildState::Superseded { .. } => BuildPhase::Superseded,
    }
}
