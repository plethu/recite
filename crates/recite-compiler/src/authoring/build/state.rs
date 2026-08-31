use super::super::freshness::FreshnessAssessment;
use super::super::identity::BuildGeneration;
use super::super::publish::{BuildCandidate, PreparedPublishIdentity};
use super::super::request::BuildRequest;
use super::super::result::{BuildResult, BuildTerminalStatus};
use super::phase::{BuildEventKind, BuildPhase};
use recite_core::Diagnostic;

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
        diagnostics: Vec<Diagnostic>,
        freshness: FreshnessAssessment,
    },
    Ready {
        request: BuildRequest,
        candidates: Vec<BuildCandidate>,
        diagnostics: Vec<Diagnostic>,
        freshness: FreshnessAssessment,
    },
    Publishing {
        request: BuildRequest,
        prepared: PreparedPublishIdentity,
        diagnostics: Vec<Diagnostic>,
        freshness: FreshnessAssessment,
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
    /// Borrow the request while the build is still active.
    #[must_use]
    pub const fn request(&self) -> Option<&BuildRequest> {
        match self {
            Self::Checking { request }
            | Self::Building { request, .. }
            | Self::Ready { request, .. }
            | Self::Publishing { request, .. } => Some(request),
            Self::Idle
            | Self::Succeeded { .. }
            | Self::Failed { .. }
            | Self::Stale { .. }
            | Self::Cancelled { .. }
            | Self::Superseded { .. } => None,
        }
    }

    /// Borrow candidates accumulated before publication or retained by a
    /// terminal result. The lifecycle owns their deterministic ordering.
    #[must_use]
    pub fn candidates(&self) -> &[BuildCandidate] {
        match self {
            Self::Building { candidates, .. } | Self::Ready { candidates, .. } => candidates,
            Self::Publishing { prepared, .. } => prepared.candidates(),
            Self::Succeeded { result }
            | Self::Failed { result }
            | Self::Stale { result }
            | Self::Cancelled { result }
            | Self::Superseded { result } => result.candidates(),
            Self::Idle | Self::Checking { .. } => &[],
        }
    }

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
    Start {
        request: BuildRequest,
    },
    CheckPassed {
        freshness: FreshnessAssessment,
        diagnostics: Vec<Diagnostic>,
    },
    CheckFailed {
        result: BuildResult,
    },
    BuildCompleted {
        candidates: Vec<BuildCandidate>,
    },
    NoCandidates {
        result: BuildResult,
    },
    PublishStarted {
        prepared: PreparedPublishIdentity,
    },
    PublishCompleted {
        result: BuildResult,
    },
    Cancelled {
        result: BuildResult,
    },
    Superseded {
        result: BuildResult,
    },
    Stale {
        result: BuildResult,
    },
    Failed {
        result: BuildResult,
    },
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
    #[error("build candidates must be ordered by target")]
    CandidatesOutOfOrder,
    #[error("prepared publication identity does not match the ready build")]
    PreparedIdentityMismatch,
    #[error("publish completion does not contain a published outcome")]
    ResultPublishMismatch,
}
