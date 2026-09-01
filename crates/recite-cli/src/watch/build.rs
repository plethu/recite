use std::io::Write;

use recite_compiler::{
    BuildControl, BuildResult, BuildResultFailure, BuildTelemetry, BuildTerminalStatus,
    FreshnessAssessment, FreshnessFailureReason, FreshnessFinalization, FreshnessStatus,
    PublishOutcome, RecoveryNeeded,
};
use recite_config::discover_project;

use super::events::WatchState;
use super::{
    ProjectBuildEngine, ProjectBuildPreparation, ProjectBuildPreparationError,
    ProjectBuildPublisher, ProjectBuildRecovery,
};
use crate::diagnostics::report_diagnostics;
use crate::error::CliError;
use crate::i18n::Messages;

mod failure;
mod failure_reasons;
mod status;

#[cfg(test)]
mod tests;

pub(super) use failure::{
    format_failure_with_recovery, format_recovery_notice, format_recovery_required,
};
pub(super) use status::BuildStatus;

pub(super) fn build_once(
    state: &mut WatchState,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<BuildStatus, CliError> {
    let control = BuildControl::new();
    build_once_with_control(state, stderr, messages, &control)
}

pub(super) fn build_once_with_control(
    state: &mut WatchState,
    stderr: &mut dyn Write,
    messages: &Messages,
    control: &BuildControl,
) -> Result<BuildStatus, CliError> {
    build_once_with_post_publish_hook(state, stderr, messages, control, || {})
}

pub(super) fn build_once_with_post_publish_hook<F>(
    state: &mut WatchState,
    stderr: &mut dyn Write,
    messages: &Messages,
    control: &BuildControl,
    post_publish: F,
) -> Result<BuildStatus, CliError>
where
    F: FnOnce(),
{
    let started_at = super::events::monotonic_now();
    let mut clock = || super::events::monotonic_now().saturating_duration_since(started_at);
    build_once_with_clock(state, stderr, messages, control, &mut clock, post_publish)
}

fn build_once_with_clock<C, F>(
    state: &mut WatchState,
    stderr: &mut dyn Write,
    messages: &Messages,
    control: &BuildControl,
    clock: &mut C,
    post_publish: F,
) -> Result<BuildStatus, CliError>
where
    C: FnMut() -> std::time::Duration,
    F: FnOnce(),
{
    // Timing belongs to the host watch invocation, not the deterministic
    // compiler lifecycle. This measures discovery, preparation, and the
    // coordinator through its terminal result; debounce and post-publish
    // freshness inspection remain outside the build duration.
    let started_at = clock();
    let generation = state.next_build_generation()?;
    let discovery = match discover_project(&state.project_root) {
        Ok(discovery) => discovery,
        Err(error) => {
            return match super::preparation::classify_discovery_error(error) {
                Ok(ProjectBuildPreparation::Rejected { diagnostics }) => {
                    report_diagnostics(stderr, messages, diagnostics.iter())?;
                    Ok(BuildStatus::Diagnostics {
                        telemetry: BuildTelemetry::from_duration(
                            clock().saturating_sub(started_at),
                        ),
                    })
                }
                Ok(ProjectBuildPreparation::Ready(_)) => Err(CliError::Watch {
                    message: "discovery error classification returned a ready request".to_owned(),
                }),
                Err(error) => Err(map_preparation_error(error)),
            };
        }
    };
    state.update_from_discovery(&discovery);
    let preparation = super::preparation::prepare_discovered(
        discovery,
        generation,
        recite_compiler::SnapshotGeneration::initial(),
    )
    .map_err(map_preparation_error)?;
    let request = match preparation {
        ProjectBuildPreparation::Ready(request) => *request,
        ProjectBuildPreparation::Rejected { diagnostics } => {
            report_diagnostics(stderr, messages, diagnostics.iter())?;
            return Ok(BuildStatus::Diagnostics {
                telemetry: BuildTelemetry::from_duration(clock().saturating_sub(started_at)),
            });
        }
    };

    let mut engine = ProjectBuildEngine::new(&request);
    let mut publisher = ProjectBuildPublisher::new(&request).map_err(|error| CliError::Watch {
        message: error.to_string(),
    })?;
    let result = match state.coordinator.run(
        request.build_request().clone(),
        control,
        &mut engine,
        &mut publisher,
    ) {
        Ok(result) => result,
        Err(source) => {
            let recovery = publisher.recovery().to_vec();
            return Err(CliError::WatchCoordinator { source, recovery });
        }
    };
    let telemetry = BuildTelemetry::from_duration(clock().saturating_sub(started_at));
    let result = result.with_telemetry(telemetry);
    let recovery = publisher.recovery().to_vec();

    if result.status() == BuildTerminalStatus::Succeeded
        && matches!(result.publish(), PublishOutcome::Published { .. })
    {
        post_publish();
        let freshness = match super::freshness::assess_current_freshness(&request) {
            Ok(freshness) => freshness,
            Err(source) => {
                report_diagnostics(stderr, messages, result.diagnostics().iter())?;
                let finalization = FreshnessFinalization::Indeterminate {
                    assessment: FreshnessAssessment::not_assessed(
                        request.build_request().fingerprints().clone(),
                    ),
                    diagnostics: Vec::new(),
                    recovery: shared_recovery(&result, &recovery),
                    reason: FreshnessFailureReason::RecheckFailed,
                };
                if let Err(source) = state.coordinator.finalize_freshness(finalization) {
                    return Err(CliError::WatchCoordinator { source, recovery });
                }
                return Err(CliError::WatchRecovery {
                    source: Box::new(source),
                    recovery,
                });
            }
        };
        let finalization = match freshness.assessment.status() {
            FreshnessStatus::Fresh => FreshnessFinalization::Fresh {
                assessment: freshness.assessment,
                diagnostics: freshness.diagnostics,
                recovery: shared_recovery(&result, &recovery),
            },
            FreshnessStatus::Stale => FreshnessFinalization::Stale {
                assessment: freshness.assessment,
                diagnostics: freshness.diagnostics,
                recovery: shared_recovery(&result, &recovery),
            },
            _ => FreshnessFinalization::Indeterminate {
                assessment: freshness.assessment,
                diagnostics: freshness.diagnostics,
                recovery: shared_recovery(&result, &recovery),
                reason: FreshnessFailureReason::RecheckFailed,
            },
        };
        let result = state
            .coordinator
            .finalize_freshness(finalization)
            .map_err(|source| CliError::WatchCoordinator {
                source,
                recovery: recovery.clone(),
            })?
            .with_telemetry(result.telemetry().clone());
        report_diagnostics(stderr, messages, result.diagnostics().iter())?;
        if result.status() == BuildTerminalStatus::Stale {
            return Ok(BuildStatus::Stale {
                asset_count: result.candidates().len(),
                recovery,
                telemetry: result.telemetry().clone(),
            });
        }
        if !result.diagnostics().is_empty() {
            return if recovery.is_empty() {
                Ok(BuildStatus::Diagnostics {
                    telemetry: result.telemetry().clone(),
                })
            } else {
                Ok(BuildStatus::DiagnosticsWithRecovery {
                    recovery,
                    telemetry: result.telemetry().clone(),
                })
            };
        }
        if !recovery.is_empty() {
            return Ok(BuildStatus::RecoveryRequired {
                asset_count: result.candidates().len(),
                recovery,
                telemetry: result.telemetry().clone(),
            });
        }
        return Ok(BuildStatus::Fresh {
            asset_count: result.candidates().len(),
            telemetry: result.telemetry().clone(),
        });
    }
    report_diagnostics(stderr, messages, result.diagnostics().iter())?;
    Ok(status_without_freshness(result, recovery))
}

fn shared_recovery(
    result: &BuildResult,
    recovery: &[ProjectBuildRecovery],
) -> Option<RecoveryNeeded> {
    if recovery.is_empty() {
        None
    } else {
        Some(RecoveryNeeded::for_targets(
            result
                .candidates()
                .iter()
                .map(|candidate| candidate.target().clone())
                .collect(),
        ))
    }
}

pub(super) fn status_without_freshness(
    result: BuildResult,
    recovery: Vec<ProjectBuildRecovery>,
) -> BuildStatus {
    let telemetry = result.telemetry().clone();
    if matches!(
        result.failure(),
        Some(BuildResultFailure::Diagnostics { .. })
    ) || matches!(
        result.publish(),
        PublishOutcome::NotAttempted {
            reason: recite_compiler::PublishNotAttemptedReason::BuildFailed
        }
    ) && result
        .diagnostics()
        .iter()
        .any(|diagnostic| diagnostic.severity == recite_core::DiagnosticSeverity::Error)
    {
        return if recovery.is_empty() {
            BuildStatus::Diagnostics { telemetry }
        } else {
            BuildStatus::DiagnosticsWithRecovery {
                recovery,
                telemetry,
            }
        };
    }

    BuildStatus::PublicationFailure {
        status: result.status(),
        failure: result.failure().cloned(),
        outcome: result.publish().clone(),
        recovery,
        telemetry,
    }
}

fn map_preparation_error(error: ProjectBuildPreparationError) -> CliError {
    match error {
        ProjectBuildPreparationError::Discovery(source) => CliError::ProjectDiscovery { source },
        ProjectBuildPreparationError::NoInputs => CliError::NoInputs,
        error => CliError::Watch {
            message: error.to_string(),
        },
    }
}
