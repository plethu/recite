mod authority;
mod candidate_order;
mod candidates;
mod coordinator;
mod execution;
mod execution_support;
mod failure;
mod fingerprints;
mod freshness;
mod identity;
mod lifecycle;
mod outcome;
mod publish;
mod request;
mod request_identity;
mod result;
mod status;

pub use authority::{BuildAuthority, BuildAuthorityError, BuildAuthorityFence, BuildPublishPermit};
pub(crate) use candidate_order::{candidates_are_ordered, compare_candidates};
pub use coordinator::{
    BuildCancellation, BuildControl, BuildCoordinator, BuildEngine, BuildFailure,
    BuildFailureReason, BuildPublisher, BuildRunError,
};
pub(crate) use execution::build_run;
pub use failure::BuildResultFailure;
pub use fingerprints::{BuildFingerprintSet, BuildInputFingerprint};
pub use freshness::{
    AffectedInput, AffectedInputReason, FreshnessAssessment, FreshnessFailureReason,
    FreshnessFinalization, FreshnessStatus, RestartGuidance, StaleReason,
};
pub use identity::{
    BuildGeneration, BuildGenerationError, BuildInput, BuildInputAuthority, BuildInputKind,
    BuildInputPayload, BuildInputPolicy,
};
pub use lifecycle::{
    BuildEventKind, BuildLifecycle, BuildPhase, BuildState, BuildTransition, BuildTransitionError,
};
pub(crate) use outcome::{finish_cancelled, make_result, normalize_publish};
pub use publish::{
    BuildCandidate, BuildPreparedHandle, BuildTarget, BuildTargetError, PreparedPublishIdentity,
    PublishAbortReason, PublishFailure, PublishFailureReason, PublishNotAttemptedReason,
    PublishOutcome, PublishOutcomeError, PublishRefusal, RecoveryNeeded,
};
pub use request::{BuildCheck, BuildCheckError, BuildRequest, BuildRequestError};
pub use request_identity::BuildRequestIdentity;
pub use result::{BuildResult, BuildTelemetry, BuildTerminalStatus};
pub use status::BuildStatusProjection;
