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

impl PartialEq for BuildStatusProjection {
    fn eq(&self, other: &Self) -> bool {
        self.semantic_eq(other)
    }
}

impl BuildStatusProjection {
    /// Compare deterministic build state while ignoring host timing metadata.
    ///
    /// [`BuildTelemetry`](super::BuildTelemetry) is retained for consumers
    /// that need measured host performance, but it must not make equivalent
    /// lifecycle projections unequal across machines or runs.
    #[must_use]
    pub fn semantic_eq(&self, other: &Self) -> bool {
        self.phase == other.phase
            && self.terminal_status == other.terminal_status
            && self.request_identity == other.request_identity
            && self.affected_inputs == other.affected_inputs
            && self.diagnostics == other.diagnostics
            && self.candidates == other.candidates
            && self.freshness == other.freshness
            && self.publish == other.publish
            && self.recovery == other.recovery
            && self.restart_guidance == other.restart_guidance
            && self.failure == other.failure
            && self.cancellation == other.cancellation
    }

    /// Non-semantic host timing metadata from a completed build, when the
    /// caller supplied it.
    #[must_use]
    pub const fn telemetry(&self) -> &super::BuildTelemetry {
        &self.telemetry
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
