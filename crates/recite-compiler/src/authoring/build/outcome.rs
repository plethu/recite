use super::coordinator::{BuildCancellation, BuildRunError};
use super::failure::BuildResultFailure;
use super::freshness::FreshnessAssessment;
use super::lifecycle::{BuildLifecycle, BuildTransition, BuildTransitionError};
use super::publish::{BuildCandidate, PublishNotAttemptedReason, PublishOutcome, RecoveryNeeded};
use super::request::BuildRequest;
use super::result::{BuildResult, BuildTerminalStatus};

pub(crate) fn normalize_publish(outcome: PublishOutcome) -> PublishOutcome {
    match outcome {
        PublishOutcome::Published { mut targets } => {
            targets.sort();
            PublishOutcome::Published { targets }
        }
        PublishOutcome::Partial {
            mut committed,
            failed,
            mut remaining,
            recovery,
        } => {
            committed.sort();
            remaining.sort();
            PublishOutcome::Partial {
                committed,
                failed,
                remaining,
                recovery: RecoveryNeeded::for_targets(recovery.targets().to_vec()),
            }
        }
        PublishOutcome::Indeterminate {
            mut attempted,
            recovery,
        } => {
            attempted.sort();
            PublishOutcome::Indeterminate {
                attempted,
                recovery: RecoveryNeeded::for_targets(recovery.targets().to_vec()),
            }
        }
        other => other,
    }
}

pub(crate) fn make_result(
    request: &BuildRequest,
    status: BuildTerminalStatus,
    diagnostics: Vec<recite_core::Diagnostic>,
    candidates: Vec<BuildCandidate>,
    freshness: FreshnessAssessment,
    publish: PublishOutcome,
    failure: Option<BuildResultFailure>,
) -> BuildResult {
    BuildResult::new(
        status,
        request,
        diagnostics,
        candidates,
        freshness,
        publish,
        failure,
    )
}

pub(crate) fn finish_cancelled(
    lifecycle: &mut BuildLifecycle,
    request: &BuildRequest,
    cancellation: BuildCancellation,
    candidates: Vec<BuildCandidate>,
    diagnostics: Vec<recite_core::Diagnostic>,
    freshness: FreshnessAssessment,
    failure: Option<BuildResultFailure>,
) -> Result<BuildResult, BuildRunError> {
    let (status, reason) = match cancellation {
        BuildCancellation::User => (
            BuildTerminalStatus::Cancelled,
            PublishNotAttemptedReason::Cancelled,
        ),
        BuildCancellation::Superseded { .. } => (
            BuildTerminalStatus::Superseded,
            PublishNotAttemptedReason::Superseded,
        ),
    };
    let result = make_result(
        request,
        status,
        diagnostics,
        candidates,
        freshness,
        PublishOutcome::NotAttempted { reason },
        failure,
    )
    .with_cancellation(cancellation);
    let transition = match status {
        BuildTerminalStatus::Cancelled => BuildTransition::Cancelled {
            result: result.clone(),
        },
        BuildTerminalStatus::Superseded => BuildTransition::Superseded {
            result: result.clone(),
        },
        BuildTerminalStatus::Succeeded
        | BuildTerminalStatus::Failed
        | BuildTerminalStatus::Stale => {
            return Err(BuildRunError::Transition(
                BuildTransitionError::ResultStatusMismatch {
                    expected: BuildTerminalStatus::Cancelled,
                    status,
                },
            ));
        }
    };
    lifecycle.transition(transition)?;
    Ok(result)
}
