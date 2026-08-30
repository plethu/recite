use std::sync::Arc;
use std::sync::atomic::{AtomicBool, AtomicU64, Ordering};

use super::authority::{BuildAuthority, BuildAuthorityError, BuildAuthorityFence};
use super::build_run;
use super::identity::BuildGeneration;
use super::lifecycle::{BuildLifecycle, BuildTransitionError};
use super::publish::{
    BuildCandidate, PreparedPublish, PublishAbortReason, PublishFailure, PublishOutcome,
};
use super::request::{BuildCheck, BuildRequest};
use super::result::BuildResult;

/// Cooperative cancellation and supersession state for one build invocation.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum BuildCancellation {
    /// The caller requested cancellation.
    User,
    /// A newer request owns the build authority.
    Superseded { by: BuildGeneration },
}

/// Mutable cancellation state shared with injected build seams.
#[derive(Debug)]
struct BuildControlState {
    cancelled: AtomicBool,
    superseded: AtomicBool,
    superseded_by: AtomicU64,
}

#[non_exhaustive]
#[derive(Clone, Debug)]
pub struct BuildControl {
    state: Arc<BuildControlState>,
}
impl BuildControl {
    /// Construct a clear control value.
    #[must_use]
    pub fn new() -> Self {
        Self {
            state: Arc::new(BuildControlState {
                cancelled: AtomicBool::new(false),
                superseded: AtomicBool::new(false),
                superseded_by: AtomicU64::new(0),
            }),
        }
    }
    /// Request user cancellation; supersession remains dominant.
    pub fn cancel(&self) {
        self.state.cancelled.store(true, Ordering::Release);
    }
    /// Mark this request superseded by a newer generation.
    pub fn supersede(&self, by: BuildGeneration) {
        self.state
            .superseded_by
            .fetch_max(by.as_u64(), Ordering::AcqRel);
        self.state.superseded.store(true, Ordering::Release);
    }
    /// Return the winning cancellation reason.
    #[must_use]
    pub fn cancellation(&self) -> Option<BuildCancellation> {
        if self.state.superseded.load(Ordering::Acquire) {
            Some(BuildCancellation::Superseded {
                by: BuildGeneration::new(self.state.superseded_by.load(Ordering::Acquire)),
            })
        } else if self.state.cancelled.load(Ordering::Acquire) {
            Some(BuildCancellation::User)
        } else {
            None
        }
    }
}

impl Default for BuildControl {
    fn default() -> Self {
        Self::new()
    }
}
impl PartialEq for BuildControl {
    fn eq(&self, other: &Self) -> bool {
        self.cancellation() == other.cancellation()
    }
}
impl Eq for BuildControl {}

/// The validation and compilation seam owned by a host or compiler caller.
pub trait BuildEngine {
    /// Validate inputs and assess current freshness.
    fn check(&mut self, request: &BuildRequest, control: &BuildControl) -> BuildCheck;
    /// Produce all unpublished candidates after a successful check.
    fn build(
        &mut self,
        request: &BuildRequest,
        control: &BuildControl,
    ) -> Result<Vec<BuildCandidate>, BuildFailure>;
}

/// The host-owned preparation and publication seam.
pub trait BuildPublisher {
    /// Stage every candidate without replacing any published target.
    fn prepare(
        &mut self,
        request: &BuildRequest,
        candidates: &[BuildCandidate],
        control: &BuildControl,
    ) -> Result<PreparedPublish, PublishFailure>;
    /// Discard staged data after cancellation or a failed preparation.
    fn abort(&mut self, prepared: Option<PreparedPublish>, reason: PublishAbortReason);
    /// Replace prepared targets; global multi-target atomicity is not promised.
    fn commit(&mut self, prepared: PreparedPublish) -> PublishOutcome;
}

/// Typed compilation failure returned by an injected engine.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum BuildFailure {
    /// Validation or compiler diagnostics made the candidate set unusable.
    #[error("build compilation returned diagnostics")]
    Diagnostics {
        diagnostics: Vec<recite_core::Diagnostic>,
    },
    /// The engine could not produce a valid candidate set.
    #[error("build engine failed: {reason}")]
    Engine { reason: BuildFailureReason },
    /// Two candidates attempted to replace the same target.
    #[error("build produced duplicate target {target}")]
    DuplicateTarget { target: super::publish::BuildTarget },
}

/// Stable engine-failure category without host error text.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum BuildFailureReason {
    InvalidOutput,
    Host,
    Unknown,
}
impl std::fmt::Display for BuildFailureReason {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(match self {
            Self::InvalidOutput => "invalid output",
            Self::Host => "host failure",
            Self::Unknown => "unknown failure",
        })
    }
}

/// Failure of the reducer itself; content failures are [`BuildResult`] values.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum BuildRunError {
    /// The coordinator could not apply an expected legal transition.
    #[error(transparent)]
    Transition(#[from] BuildTransitionError),
    #[error(transparent)]
    Authority(#[from] BuildAuthorityError),
    #[error("coordinator has no publication authority")]
    MissingAuthority,
}

/// Synchronous orchestration around the pure lifecycle reducer.
#[non_exhaustive]
#[derive(Clone, Debug, Default)]
pub struct BuildCoordinator {
    lifecycle: BuildLifecycle,
    authority: Option<BuildAuthorityFence>,
}
impl BuildCoordinator {
    /// Construct an idle coordinator.
    #[must_use]
    pub const fn new() -> Self {
        Self {
            lifecycle: BuildLifecycle::new(),
            authority: None,
        }
    }
    /// Construct a coordinator sharing a live publication authority.
    #[must_use]
    pub fn with_fence(fence: BuildAuthorityFence) -> Self {
        Self {
            lifecycle: BuildLifecycle::new(),
            authority: Some(fence),
        }
    }
    /// Borrow the reducer state after the most recent run.
    #[must_use]
    pub const fn state(&self) -> &super::lifecycle::BuildState {
        self.lifecycle.state()
    }
    /// Run a request under this coordinator's live publication authority.
    pub fn run<E: BuildEngine, P: BuildPublisher>(
        &mut self,
        request: BuildRequest,
        control: &BuildControl,
        engine: &mut E,
        publisher: &mut P,
    ) -> Result<BuildResult, BuildRunError> {
        if self.authority.is_none() {
            self.authority = Some(BuildAuthorityFence::new(BuildAuthority::from_request(
                &request,
            )));
        }
        let fence = match self.authority.as_ref() {
            Some(fence) => fence,
            None => return Err(BuildRunError::MissingAuthority),
        };
        fence.install_if_newer(BuildAuthority::from_request(&request))?;
        build_run(
            &mut self.lifecycle,
            request,
            control,
            fence,
            engine,
            publisher,
        )
    }
}
