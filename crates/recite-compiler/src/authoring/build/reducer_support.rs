use super::super::phase::{BuildEventKind, BuildPhase};
use super::super::state::{BuildState, BuildTransitionError};

pub(crate) fn invalid(state: &BuildState, event: BuildEventKind) -> BuildTransitionError {
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
        BuildState::Ready { .. } => BuildPhase::Ready,
        BuildState::Succeeded { .. } => BuildPhase::Succeeded,
        BuildState::Failed { .. } => BuildPhase::Failed,
        BuildState::Stale { .. } => BuildPhase::Stale,
        BuildState::Cancelled { .. } => BuildPhase::Cancelled,
        BuildState::Superseded { .. } => BuildPhase::Superseded,
    }
}
