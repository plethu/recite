mod coordinator;
mod execution;
mod freshness;
mod identity;
mod lifecycle;
mod outcome;
mod publish;
mod request;
mod result;

pub use coordinator::{
    BuildCancellation, BuildControl, BuildCoordinator, BuildEngine, BuildFailure,
    BuildFailureReason, BuildPublisher, BuildRunError,
};
pub(crate) use execution::build_run;
pub use freshness::{
    AffectedInput, AffectedInputReason, FreshnessAssessment, FreshnessStatus, RestartGuidance,
    StaleReason,
};
pub use identity::{
    BuildFingerprintSet, BuildGeneration, BuildGenerationError, BuildInput, BuildInputAuthority,
    BuildInputFingerprint, BuildInputKind, BuildInputPolicy,
};
pub use lifecycle::{
    BuildEventKind, BuildLifecycle, BuildPhase, BuildState, BuildTransition, BuildTransitionError,
};
pub(crate) use outcome::{
    authority_refusal, cancellation_result, finish_cancelled, make_result, normalize_publish,
};
pub use publish::{
    BuildCandidate, BuildTarget, BuildTargetError, PublishFailure, PublishFailureReason,
    PublishNotAttemptedReason, PublishOutcome, PublishRefusal, RecoveryNeeded,
};
pub use request::{BuildCheck, BuildRequest, BuildRequestError};
pub use result::{BuildAuthority, BuildResult, BuildTelemetry, BuildTerminalStatus};
