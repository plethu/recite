use super::coordinator::{BuildCancellation, BuildControl, BuildRunError};
use super::freshness::FreshnessAssessment;
use super::lifecycle::{BuildLifecycle, BuildTransition, BuildTransitionError};
use super::publish::{BuildCandidate, PublishNotAttemptedReason, PublishOutcome, RecoveryNeeded};
use super::request::BuildRequest;
use super::result::{BuildAuthority, BuildResult, BuildTelemetry, BuildTerminalStatus};

pub(crate) fn authority_refusal<A: FnMut() -> BuildAuthority>(
    authority: &mut A,
    request: &BuildRequest,
) -> Option<super::publish::PublishRefusal> {
    authority().refusal_for(request)
}

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
) -> BuildResult {
    BuildResult::new(
        status,
        request,
        diagnostics,
        candidates,
        freshness,
        publish,
        BuildTelemetry::none(),
    )
}

pub(crate) fn finish_cancelled(
    lifecycle: &mut BuildLifecycle,
    request: &BuildRequest,
    cancellation: BuildCancellation,
    candidates: Vec<BuildCandidate>,
    freshness: FreshnessAssessment,
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
        Vec::new(),
        candidates,
        freshness,
        PublishOutcome::NotAttempted { reason },
    );
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

pub(crate) fn cancellation_result(
    lifecycle: &mut BuildLifecycle,
    request: &BuildRequest,
    control: &BuildControl,
    candidates: &[BuildCandidate],
    freshness: FreshnessAssessment,
) -> Result<Option<BuildResult>, BuildRunError> {
    control
        .cancellation()
        .map(|reason| finish_cancelled(lifecycle, request, reason, candidates.to_vec(), freshness))
        .transpose()
}
