use super::coordinator::{BuildControl, BuildEngine, BuildFailure, BuildPublisher, BuildRunError};
use super::freshness::FreshnessAssessment;
use super::lifecycle::{BuildLifecycle, BuildTransition};
use super::publish::{PublishFailure, PublishNotAttemptedReason, PublishOutcome, PublishRefusal};
use super::request::BuildRequest;
use super::result::{BuildResult, BuildTerminalStatus};
use super::{
    authority_refusal, cancellation_result, finish_cancelled, make_result, normalize_publish,
};

pub(crate) fn build_run<
    E: BuildEngine,
    P: BuildPublisher,
    A: FnMut() -> super::result::BuildAuthority,
>(
    lifecycle: &mut BuildLifecycle,
    request: BuildRequest,
    control: &BuildControl,
    mut authority: A,
    engine: &mut E,
    publisher: &mut P,
) -> Result<BuildResult, BuildRunError> {
    let not_assessed = FreshnessAssessment::not_assessed(request.fingerprints().clone());
    lifecycle.transition(BuildTransition::Start {
        request: request.clone(),
    })?;
    if let Some(reason) = control.cancellation() {
        return finish_cancelled(lifecycle, &request, reason, Vec::new(), not_assessed);
    }

    let check = engine.check(&request, control);
    if let Some(reason) = control.cancellation() {
        return finish_cancelled(
            lifecycle,
            &request,
            reason,
            Vec::new(),
            check.freshness().clone(),
        );
    }
    if !check.is_valid() {
        let result = make_result(
            &request,
            BuildTerminalStatus::Failed,
            check.diagnostics().to_vec(),
            Vec::new(),
            check.freshness().clone(),
            PublishOutcome::NotAttempted {
                reason: PublishNotAttemptedReason::BuildFailed,
            },
        );
        lifecycle.transition(BuildTransition::CheckFailed {
            result: result.clone(),
        })?;
        return Ok(result);
    }
    lifecycle.transition(BuildTransition::CheckPassed {
        freshness: check.freshness().clone(),
    })?;

    let mut candidates = match engine.build(&request, control) {
        Ok(candidates) => candidates,
        Err(failure) => {
            let diagnostics = match failure {
                BuildFailure::Diagnostics { diagnostics } => diagnostics,
                BuildFailure::Engine { .. } | BuildFailure::DuplicateTarget { .. } => Vec::new(),
            };
            let result = make_result(
                &request,
                BuildTerminalStatus::Failed,
                diagnostics,
                Vec::new(),
                check.freshness().clone(),
                PublishOutcome::NotAttempted {
                    reason: PublishNotAttemptedReason::BuildFailed,
                },
            );
            lifecycle.transition(BuildTransition::Failed {
                result: result.clone(),
            })?;
            return Ok(result);
        }
    };
    candidates.sort_by(|left, right| left.target().cmp(right.target()));
    if candidates
        .windows(2)
        .any(|window| window[0].target() == window[1].target())
    {
        let result = make_result(
            &request,
            BuildTerminalStatus::Failed,
            Vec::new(),
            candidates,
            check.freshness().clone(),
            PublishOutcome::NotAttempted {
                reason: PublishNotAttemptedReason::BuildFailed,
            },
        );
        lifecycle.transition(BuildTransition::Failed {
            result: result.clone(),
        })?;
        return Ok(result);
    }
    if let Some(reason) = control.cancellation() {
        return finish_cancelled(
            lifecycle,
            &request,
            reason,
            candidates,
            check.freshness().clone(),
        );
    }
    lifecycle.transition(BuildTransition::BuildCompleted {
        candidates: candidates.clone(),
    })?;

    for candidate in &candidates {
        if let Some(reason) = control.cancellation() {
            return finish_cancelled(
                lifecycle,
                &request,
                reason,
                candidates,
                check.freshness().clone(),
            );
        }
        let preparation = publisher.prepare(candidate, control);
        if let Some(reason) = control.cancellation() {
            return finish_cancelled(
                lifecycle,
                &request,
                reason,
                candidates,
                check.freshness().clone(),
            );
        }
        if let Err(failure) = preparation {
            let reason = match failure {
                PublishFailure::Preparation { .. } => PublishNotAttemptedReason::PreparationFailed,
            };
            let result = make_result(
                &request,
                BuildTerminalStatus::Failed,
                Vec::new(),
                candidates,
                check.freshness().clone(),
                PublishOutcome::NotAttempted { reason },
            );
            lifecycle.transition(BuildTransition::Failed {
                result: result.clone(),
            })?;
            return Ok(result);
        }
    }
    lifecycle.transition(BuildTransition::PublishStarted)?;
    if let Some(reason) = control.cancellation() {
        return finish_cancelled(
            lifecycle,
            &request,
            reason,
            candidates,
            check.freshness().clone(),
        );
    }
    if candidates.is_empty() {
        let result = make_result(
            &request,
            BuildTerminalStatus::Succeeded,
            check.diagnostics().to_vec(),
            candidates,
            check.freshness().clone(),
            PublishOutcome::NotAttempted {
                reason: PublishNotAttemptedReason::NoCandidates,
            },
        );
        lifecycle.transition(BuildTransition::PublishCompleted {
            result: result.clone(),
        })?;
        return Ok(result);
    }
    if let Some(refusal) = authority_refusal(&mut authority, &request) {
        let result = make_result(
            &request,
            BuildTerminalStatus::Stale,
            check.diagnostics().to_vec(),
            candidates,
            check.freshness().clone(),
            PublishOutcome::Refused { reason: refusal },
        );
        lifecycle.transition(BuildTransition::Stale {
            result: result.clone(),
        })?;
        return Ok(result);
    }
    if let Some(result) = cancellation_result(
        lifecycle,
        &request,
        control,
        &candidates,
        check.freshness().clone(),
    )? {
        return Ok(result);
    }
    let publish = normalize_publish(publisher.commit(&candidates));
    let status = match publish {
        PublishOutcome::Published { .. } => BuildTerminalStatus::Succeeded,
        PublishOutcome::Refused {
            reason:
                PublishRefusal::StaleBuildGeneration
                | PublishRefusal::StaleSnapshotGeneration
                | PublishRefusal::StaleFingerprints,
        } => BuildTerminalStatus::Stale,
        PublishOutcome::NotAttempted {
            reason: PublishNotAttemptedReason::Stale,
        } => BuildTerminalStatus::Stale,
        PublishOutcome::NotAttempted { .. } | PublishOutcome::Partial { .. } => {
            BuildTerminalStatus::Failed
        }
    };
    let result = make_result(
        &request,
        status,
        check.diagnostics().to_vec(),
        candidates,
        check.freshness().clone(),
        publish,
    );
    let transition = match status {
        BuildTerminalStatus::Succeeded => BuildTransition::PublishCompleted {
            result: result.clone(),
        },
        BuildTerminalStatus::Stale => BuildTransition::Stale {
            result: result.clone(),
        },
        BuildTerminalStatus::Failed => BuildTransition::Failed {
            result: result.clone(),
        },
        BuildTerminalStatus::Cancelled | BuildTerminalStatus::Superseded => {
            return Err(super::coordinator::BuildRunError::Transition(
                super::lifecycle::BuildTransitionError::ResultStatusMismatch {
                    expected: BuildTerminalStatus::Failed,
                    status,
                },
            ));
        }
    };
    lifecycle.transition(transition)?;
    Ok(result)
}
