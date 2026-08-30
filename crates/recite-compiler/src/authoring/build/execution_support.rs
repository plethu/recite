use super::coordinator::{BuildCancellation, BuildFailure, BuildRunError};
use super::failure::BuildResultFailure;
use super::freshness::FreshnessAssessment;
use super::lifecycle::{BuildLifecycle, BuildTransition};
use super::make_result;
use super::publish::{
    BuildCandidate, BuildTarget, PreparedPublishIdentity, PublishAbortReason,
    PublishNotAttemptedReason, PublishOutcome, PublishRefusal,
};
use super::request::{BuildCheck, BuildCheckError, BuildRequest};
use super::result::{BuildResult, BuildTerminalStatus};

pub(crate) fn duplicate_target(candidates: &[BuildCandidate]) -> Option<BuildTarget> {
    candidates
        .windows(2)
        .find(|pair| pair[0].target() == pair[1].target())
        .map(|pair| pair[0].target().clone())
}
pub(crate) fn failure_detail(failure: &BuildFailure) -> BuildResultFailure {
    match failure {
        BuildFailure::Diagnostics { diagnostics } => BuildResultFailure::Diagnostics {
            diagnostics: diagnostics.clone(),
        },
        BuildFailure::Engine { reason } => BuildResultFailure::Engine { reason: *reason },
        BuildFailure::DuplicateTarget { target } => BuildResultFailure::DuplicateTarget {
            target: target.clone(),
        },
    }
}
pub(crate) fn abort_reason(reason: BuildCancellation) -> PublishAbortReason {
    match reason {
        BuildCancellation::User => PublishAbortReason::Cancelled,
        BuildCancellation::Superseded { .. } => PublishAbortReason::Superseded,
    }
}
pub(crate) fn fail_check(
    lifecycle: &mut BuildLifecycle,
    request: &BuildRequest,
    freshness: FreshnessAssessment,
    error: BuildCheckError,
) -> Result<BuildResult, BuildRunError> {
    let result = make_result(
        request,
        BuildTerminalStatus::Failed,
        Vec::new(),
        Vec::new(),
        freshness,
        PublishOutcome::NotAttempted {
            reason: PublishNotAttemptedReason::BuildFailed,
        },
        Some(BuildResultFailure::Check(error)),
    );
    lifecycle.transition(BuildTransition::CheckFailed {
        result: result.clone(),
    })?;
    Ok(result)
}
pub(crate) fn finish_failed(
    lifecycle: &mut BuildLifecycle,
    request: &BuildRequest,
    diagnostics: Vec<recite_core::Diagnostic>,
    candidates: Vec<BuildCandidate>,
    freshness: FreshnessAssessment,
    failure: BuildResultFailure,
) -> Result<BuildResult, BuildRunError> {
    let result = make_result(
        request,
        BuildTerminalStatus::Failed,
        diagnostics,
        candidates,
        freshness,
        PublishOutcome::NotAttempted {
            reason: PublishNotAttemptedReason::BuildFailed,
        },
        Some(failure),
    );
    lifecycle.transition(BuildTransition::Failed {
        result: result.clone(),
    })?;
    Ok(result)
}
pub(crate) fn finish_stale(
    lifecycle: &mut BuildLifecycle,
    request: &BuildRequest,
    candidates: Vec<BuildCandidate>,
    freshness: FreshnessAssessment,
    reason: PublishRefusal,
) -> Result<BuildResult, BuildRunError> {
    let result = make_result(
        request,
        BuildTerminalStatus::Stale,
        Vec::new(),
        candidates,
        freshness,
        PublishOutcome::Refused { reason },
        None,
    );
    lifecycle.transition(BuildTransition::Stale {
        result: result.clone(),
    })?;
    Ok(result)
}
pub(crate) fn finish_publish(
    lifecycle: &mut BuildLifecycle,
    request: &BuildRequest,
    check: &BuildCheck,
    candidates: Vec<BuildCandidate>,
    identity: PreparedPublishIdentity,
    publish: PublishOutcome,
) -> Result<BuildResult, BuildRunError> {
    if let Err(error) = publish.validate_against(&identity) {
        let result = make_result(
            request,
            BuildTerminalStatus::Failed,
            check.diagnostics().to_vec(),
            candidates,
            check.freshness().clone(),
            PublishOutcome::NotAttempted {
                reason: PublishNotAttemptedReason::InvalidOutcome,
            },
            Some(BuildResultFailure::InvalidPublication(error)),
        );
        lifecycle.transition(BuildTransition::Failed {
            result: result.clone(),
        })?;
        return Ok(result);
    }
    let status = match &publish {
        PublishOutcome::Published { .. } => BuildTerminalStatus::Succeeded,
        PublishOutcome::Partial { .. } | PublishOutcome::NotAttempted { .. } => {
            BuildTerminalStatus::Failed
        }
        PublishOutcome::Refused {
            reason:
                PublishRefusal::StaleBuildGeneration
                | PublishRefusal::StaleSnapshotGeneration
                | PublishRefusal::StaleFingerprints,
        } => BuildTerminalStatus::Stale,
    };
    let result = make_result(
        request,
        status,
        check.diagnostics().to_vec(),
        candidates,
        check.freshness().clone(),
        publish,
        None,
    );
    let event = match status {
        BuildTerminalStatus::Succeeded => BuildTransition::PublishCompleted {
            result: result.clone(),
        },
        BuildTerminalStatus::Failed => BuildTransition::Failed {
            result: result.clone(),
        },
        BuildTerminalStatus::Stale => BuildTransition::Stale {
            result: result.clone(),
        },
        BuildTerminalStatus::Cancelled | BuildTerminalStatus::Superseded => {
            return Err(BuildRunError::Transition(
                super::lifecycle::BuildTransitionError::ResultStatusMismatch {
                    expected: BuildTerminalStatus::Failed,
                    status,
                },
            ));
        }
    };
    lifecycle.transition(event)?;
    Ok(result)
}
