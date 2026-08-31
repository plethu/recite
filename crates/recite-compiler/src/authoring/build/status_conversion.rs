use super::{BuildPhase, BuildResult, BuildState, BuildStatusProjection, BuildTerminalStatus};

impl BuildStatusProjection {
    /// Project a completed result, deriving its terminal lifecycle phase from
    /// the result's typed status.
    #[must_use]
    pub fn from_result(result: &BuildResult) -> Self {
        Self::terminal(phase_for(result.status()), result)
    }
}

impl From<&BuildState> for BuildStatusProjection {
    fn from(state: &BuildState) -> Self {
        Self::from_state(state)
    }
}

impl From<&BuildResult> for BuildStatusProjection {
    fn from(result: &BuildResult) -> Self {
        Self::from_result(result)
    }
}

const fn phase_for(status: BuildTerminalStatus) -> BuildPhase {
    match status {
        BuildTerminalStatus::Succeeded => BuildPhase::Succeeded,
        BuildTerminalStatus::Failed => BuildPhase::Failed,
        BuildTerminalStatus::Stale => BuildPhase::Stale,
        BuildTerminalStatus::Cancelled => BuildPhase::Cancelled,
        BuildTerminalStatus::Superseded => BuildPhase::Superseded,
    }
}
