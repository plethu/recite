use super::publish::{BuildTarget, PublishFailureReason, PublishOutcomeError};
use super::request::BuildCheckError;

/// Structured failure detail retained in the deterministic build result.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum BuildResultFailure {
    #[error("build check was invalid: {0}")]
    Check(#[from] BuildCheckError),
    #[error("engine returned diagnostics")]
    Diagnostics {
        diagnostics: Vec<recite_core::Diagnostic>,
    },
    #[error("engine failed: {reason}")]
    Engine {
        reason: super::coordinator::BuildFailureReason,
    },
    #[error("duplicate build target {target}")]
    DuplicateTarget { target: BuildTarget },
    #[error("could not prepare {target}: {reason}")]
    Preparation {
        target: BuildTarget,
        reason: PublishFailureReason,
    },
    #[error("publisher returned an invalid outcome: {0}")]
    InvalidPublication(#[from] PublishOutcomeError),
    #[error("post-publication freshness recheck failed: {reason}")]
    Freshness {
        reason: super::freshness::FreshnessFailureReason,
    },
}
