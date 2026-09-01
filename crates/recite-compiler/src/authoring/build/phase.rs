/// Current phase of the shared build lifecycle.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum BuildPhase {
    Idle,
    Checking,
    Building,
    Publishing,
    Ready,
    Succeeded,
    Failed,
    Stale,
    Cancelled,
    Superseded,
}
impl std::fmt::Display for BuildPhase {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Idle => "idle",
            Self::Checking => "checking",
            Self::Building => "building",
            Self::Publishing => "publishing",
            Self::Ready => "ready",
            Self::Succeeded => "succeeded",
            Self::Failed => "failed",
            Self::Stale => "stale",
            Self::Cancelled => "cancelled",
            Self::Superseded => "superseded",
        })
    }
}

/// Event accepted by the pure reducer.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum BuildEventKind {
    Start,
    CheckPassed,
    CheckFailed,
    BuildCompleted,
    NoCandidates,
    PublishStarted,
    PublishCompleted,
    Cancelled,
    Superseded,
    Stale,
    Failed,
    FreshnessFinalized,
}
impl std::fmt::Display for BuildEventKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Start => "start",
            Self::CheckPassed => "check-passed",
            Self::CheckFailed => "check-failed",
            Self::BuildCompleted => "build-completed",
            Self::NoCandidates => "no-candidates",
            Self::PublishStarted => "publish-started",
            Self::PublishCompleted => "publish-completed",
            Self::Cancelled => "cancelled",
            Self::Superseded => "superseded",
            Self::Stale => "stale",
            Self::Failed => "failed",
            Self::FreshnessFinalized => "freshness-finalized",
        })
    }
}
