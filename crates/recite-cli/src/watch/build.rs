use std::io::Write;

use recite_compiler::{
    BuildControl, BuildResult, BuildResultFailure, BuildTerminalStatus, PublishOutcome,
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

#[cfg(test)]
mod tests;

pub(super) use failure::{
    format_failure_with_recovery, format_recovery_notice, format_recovery_required,
};

/// The presentation boundary's compact view of one coordinated build.
#[derive(Debug, Eq, PartialEq)]
pub(super) enum BuildStatus {
    Fresh {
        asset_count: usize,
    },
    Stale {
        asset_count: usize,
        recovery: Vec<ProjectBuildRecovery>,
    },
    Diagnostics,
    DiagnosticsWithRecovery {
        recovery: Vec<ProjectBuildRecovery>,
    },
    RecoveryRequired {
        asset_count: usize,
        recovery: Vec<ProjectBuildRecovery>,
    },
    PublicationFailure {
        status: BuildTerminalStatus,
        failure: Option<BuildResultFailure>,
        outcome: PublishOutcome,
        recovery: Vec<ProjectBuildRecovery>,
    },
}

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
    let generation = state.next_build_generation()?;
    let discovery = match discover_project(&state.project_root) {
        Ok(discovery) => discovery,
        Err(error) => {
            return match super::preparation::classify_discovery_error(error) {
                Ok(ProjectBuildPreparation::Rejected { diagnostics }) => {
                    report_diagnostics(stderr, messages, diagnostics.iter())?;
                    Ok(BuildStatus::Diagnostics)
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
            return Ok(BuildStatus::Diagnostics);
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
    let recovery = publisher.recovery().to_vec();

    report_diagnostics(stderr, messages, result.diagnostics().iter())?;
    if result.status() == BuildTerminalStatus::Succeeded {
        post_publish();
        let freshness = match super::freshness::assess_current_freshness(&request) {
            Ok(freshness) => freshness,
            Err(source) => {
                return Err(CliError::WatchRecovery {
                    source: Box::new(source),
                    recovery,
                });
            }
        };
        report_diagnostics(stderr, messages, freshness.diagnostics.iter())?;
        if freshness.stale {
            return Ok(BuildStatus::Stale {
                asset_count: result.candidates().len(),
                recovery,
            });
        }
        if !freshness.diagnostics.is_empty() {
            return if recovery.is_empty() {
                Ok(BuildStatus::Diagnostics)
            } else {
                Ok(BuildStatus::DiagnosticsWithRecovery { recovery })
            };
        }
        if !recovery.is_empty() {
            return Ok(BuildStatus::RecoveryRequired {
                asset_count: result.candidates().len(),
                recovery,
            });
        }
        return Ok(BuildStatus::Fresh {
            asset_count: result.candidates().len(),
        });
    }
    Ok(status_without_freshness(result, recovery))
}

pub(super) fn status_without_freshness(
    result: BuildResult,
    recovery: Vec<ProjectBuildRecovery>,
) -> BuildStatus {
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
            BuildStatus::Diagnostics
        } else {
            BuildStatus::DiagnosticsWithRecovery { recovery }
        };
    }

    BuildStatus::PublicationFailure {
        status: result.status(),
        failure: result.failure().cloned(),
        outcome: result.publish().clone(),
        recovery,
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
