use super::authority::{BuildAuthorityCommitError, BuildAuthorityFence};
use super::compare_candidates;
use super::coordinator::{BuildControl, BuildEngine, BuildFailure, BuildPublisher, BuildRunError};
use super::execution_support::{
    abort_reason, duplicate_target, fail_check, failure_detail, finish_cancelled_after_check,
    finish_failed, finish_publish, finish_stale_after_check,
};
use super::failure::BuildResultFailure;
use super::freshness::FreshnessAssessment;
use super::lifecycle::{BuildLifecycle, BuildTransition};
use super::publish::{
    BuildPreparedHandle, PublishAbortReason, PublishFailure, PublishNotAttemptedReason,
    PublishOutcome,
};
use super::request::BuildRequest;
use super::result::{BuildResult, BuildTerminalStatus};
use super::{finish_cancelled, make_result, normalize_publish};

pub(crate) fn build_run<E: BuildEngine, P: BuildPublisher>(
    lifecycle: &mut BuildLifecycle,
    request: BuildRequest,
    control: &BuildControl,
    fence: &BuildAuthorityFence,
    engine: &mut E,
    publisher: &mut P,
) -> Result<BuildResult, BuildRunError> {
    let not_assessed = FreshnessAssessment::not_assessed(request.fingerprints().clone());
    lifecycle.transition(BuildTransition::Start {
        request: request.clone(),
    })?;
    if let Some(reason) = control.cancellation() {
        return finish_cancelled(
            lifecycle,
            &request,
            reason,
            Vec::new(),
            Vec::new(),
            not_assessed,
            None,
        );
    }
    let check = engine.check(&request, control);
    if let Some(reason) = control.cancellation() {
        return finish_cancelled(
            lifecycle,
            &request,
            reason,
            Vec::new(),
            check.diagnostics().to_vec(),
            check.freshness().clone(),
            None,
        );
    }
    if let Err(error) = check.validate_for(&request) {
        return fail_check(lifecycle, &request, check.freshness().clone(), error);
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
            Some(BuildResultFailure::Diagnostics {
                diagnostics: check.diagnostics().to_vec(),
            }),
        );
        lifecycle.transition(BuildTransition::CheckFailed {
            result: result.clone(),
        })?;
        return Ok(result);
    }
    let check_diagnostics = check.diagnostics().to_vec();
    lifecycle.transition(BuildTransition::CheckPassed {
        freshness: check.freshness().clone(),
        diagnostics: check_diagnostics.clone(),
    })?;
    let mut candidates = match engine.build(&request, control) {
        Ok(candidates) => candidates,
        Err(failure) => {
            if let Some(reason) = control.cancellation() {
                return finish_cancelled_after_check(
                    lifecycle,
                    &request,
                    reason,
                    Vec::new(),
                    &check,
                );
            }
            let detail = failure_detail(&failure);
            let mut diagnostics = check_diagnostics.clone();
            if let BuildFailure::Diagnostics {
                diagnostics: build_diagnostics,
            } = &failure
            {
                diagnostics.extend(build_diagnostics.iter().cloned());
            }
            return finish_failed(
                lifecycle,
                &request,
                diagnostics,
                Vec::new(),
                check.freshness().clone(),
                detail,
            );
        }
    };
    candidates.sort_by(compare_candidates);
    lifecycle.transition(BuildTransition::BuildCompleted {
        candidates: candidates.clone(),
    })?;
    if let Some(reason) = control.cancellation() {
        return finish_cancelled_after_check(lifecycle, &request, reason, candidates, &check);
    }
    if let Some(duplicate) = duplicate_target(&candidates) {
        return finish_failed(
            lifecycle,
            &request,
            check_diagnostics.clone(),
            candidates,
            check.freshness().clone(),
            BuildResultFailure::DuplicateTarget { target: duplicate },
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
            None,
        );
        lifecycle.transition(BuildTransition::NoCandidates {
            result: result.clone(),
        })?;
        return Ok(result);
    }
    let prepared = match publisher.prepare(&request, &candidates, control) {
        Ok(prepared) => prepared,
        Err(failure) => {
            publisher.abort(None, PublishAbortReason::PreparationFailed);
            if let Some(reason) = control.cancellation() {
                return finish_cancelled_after_check(
                    lifecycle, &request, reason, candidates, &check,
                );
            }
            let (target, reason) = match failure {
                PublishFailure::Preparation { target, reason } => (target, reason),
            };
            return finish_failed(
                lifecycle,
                &request,
                check_diagnostics.clone(),
                candidates,
                check.freshness().clone(),
                BuildResultFailure::Preparation { target, reason },
            );
        }
    };
    if let Some(reason) = control.cancellation() {
        publisher.abort(Some(prepared), abort_reason(reason));
        return finish_cancelled_after_check(lifecycle, &request, reason, candidates, &check);
    }
    let identity = prepared.identity();
    if let Err(error) = lifecycle.transition(BuildTransition::PublishStarted {
        prepared: identity.clone(),
    }) {
        publisher.abort(Some(prepared), PublishAbortReason::Invalid);
        return Err(error.into());
    }
    if let Some(reason) = control.cancellation() {
        publisher.abort(Some(prepared), abort_reason(reason));
        return finish_cancelled_after_check(lifecycle, &request, reason, candidates, &check);
    }
    let permit = match fence.acquire(&request) {
        Ok(permit) => permit,
        Err(error) => {
            publisher.abort(Some(prepared), PublishAbortReason::Stale);
            let reason = match error {
                super::authority::BuildAuthorityError::Refused { reason } => reason,
                error => return Err(error.into()),
            };
            return finish_stale_after_check(lifecycle, &request, candidates, &check, reason);
        }
    };
    if let Some(reason) = control.cancellation() {
        publisher.abort(Some(prepared), abort_reason(reason));
        return finish_cancelled_after_check(lifecycle, &request, reason, candidates, &check);
    }
    // The permit holds the fence lock through commit. Cancellation is deliberately
    // ineffective once this boundary is acquired; a host syscall cannot be stopped.
    let publish = match permit.commit(prepared, |prepared| publisher.commit(prepared)) {
        Ok(outcome) => normalize_publish(outcome),
        Err(BuildAuthorityCommitError::Refused { reason, prepared }) => {
            publisher.abort(Some(prepared), PublishAbortReason::Stale);
            return finish_stale_after_check(lifecycle, &request, candidates, &check, reason);
        }
        Err(BuildAuthorityCommitError::Poisoned { prepared }) => {
            publisher.abort(Some(prepared), PublishAbortReason::Invalid);
            return Err(BuildRunError::Authority(
                super::authority::BuildAuthorityError::Poisoned,
            ));
        }
    };
    finish_publish(lifecycle, &request, &check, candidates, identity, publish)
}
