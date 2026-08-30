use super::super::freshness::FreshnessAssessment;
use super::super::identity::BuildGeneration;
use super::super::publish::{BuildCandidate, PreparedPublishIdentity};
use super::super::request::BuildRequest;
use super::super::result::{BuildResult, BuildTerminalStatus};

/// Current phase of the shared build lifecycle.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[non_exhaustive]
pub enum BuildState {
    #[default]
    Idle,
    Checking {
        request: BuildRequest,
    },
    Building {
        request: BuildRequest,
        candidates: Vec<BuildCandidate>,
    },
    Ready {
        request: BuildRequest,
        candidates: Vec<BuildCandidate>,
    },
    Publishing {
        request: BuildRequest,
        prepared: PreparedPublishIdentity,
    },
    Succeeded {
        result: BuildResult,
    },
    Failed {
        result: BuildResult,
    },
    Stale {
        result: BuildResult,
    },
    Cancelled {
        result: BuildResult,
    },
    Superseded {
        result: BuildResult,
    },
}
impl BuildState {
    #[must_use]
    pub const fn generation(&self) -> Option<BuildGeneration> {
        match self {
            Self::Idle => None,
            Self::Checking { request }
            | Self::Building { request, .. }
            | Self::Ready { request, .. }
            | Self::Publishing { request, .. } => Some(request.generation()),
            Self::Succeeded { result }
            | Self::Failed { result }
            | Self::Stale { result }
            | Self::Cancelled { result }
            | Self::Superseded { result } => Some(result.generation()),
        }
    }
    #[must_use]
    pub const fn is_terminal(&self) -> bool {
        matches!(
            self,
            Self::Succeeded { .. }
                | Self::Failed { .. }
                | Self::Stale { .. }
                | Self::Cancelled { .. }
                | Self::Superseded { .. }
        )
    }
    #[must_use]
    pub const fn result(&self) -> Option<&BuildResult> {
        match self {
            Self::Succeeded { result }
            | Self::Failed { result }
            | Self::Stale { result }
            | Self::Cancelled { result }
            | Self::Superseded { result } => Some(result),
            Self::Idle
            | Self::Checking { .. }
            | Self::Building { .. }
            | Self::Ready { .. }
            | Self::Publishing { .. } => None,
        }
    }
}

/// Event accepted by the pure reducer.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum BuildTransition {
    Start { request: BuildRequest },
    CheckPassed { freshness: FreshnessAssessment },
    CheckFailed { result: BuildResult },
    BuildCompleted { candidates: Vec<BuildCandidate> },
    PublishStarted { prepared: PreparedPublishIdentity },
    PublishCompleted { result: BuildResult },
    Cancelled { result: BuildResult },
    Superseded { result: BuildResult },
    Stale { result: BuildResult },
    Failed { result: BuildResult },
}

/// Illegal reducer event or result.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum BuildTransitionError {
    #[error("cannot apply {event} while build is {state}")]
    Invalid {
        state: BuildPhase,
        event: BuildEventKind,
    },
    #[error("build generation {received} is not newer than {previous}")]
    GenerationNotNewer {
        previous: BuildGeneration,
        received: BuildGeneration,
    },
    #[error("build result generation {received} does not match active generation {active}")]
    ResultGenerationMismatch {
        active: BuildGeneration,
        received: BuildGeneration,
    },
    #[error("build result has status {status}, expected {expected}")]
    ResultStatusMismatch {
        expected: BuildTerminalStatus,
        status: BuildTerminalStatus,
    },
    #[error("build result identity does not match the active request")]
    ResultIdentityMismatch,
    #[error("build result candidates do not match the active prepared batch")]
    ResultCandidatesMismatch,
    #[error("build freshness does not match the active request")]
    FreshnessMismatch,
    #[error("prepared publication identity does not match the ready build")]
    PreparedIdentityMismatch,
    #[error("publish completion does not contain a published outcome")]
    ResultPublishMismatch,
}

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

#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum BuildEventKind {
    Start,
    CheckPassed,
    CheckFailed,
    BuildCompleted,
    PublishStarted,
    PublishCompleted,
    Cancelled,
    Superseded,
    Stale,
    Failed,
}
impl std::fmt::Display for BuildEventKind {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::Start => "start",
            Self::CheckPassed => "check-passed",
            Self::CheckFailed => "check-failed",
            Self::BuildCompleted => "build-completed",
            Self::PublishStarted => "publish-started",
            Self::PublishCompleted => "publish-completed",
            Self::Cancelled => "cancelled",
            Self::Superseded => "superseded",
            Self::Stale => "stale",
            Self::Failed => "failed",
        })
    }
}
