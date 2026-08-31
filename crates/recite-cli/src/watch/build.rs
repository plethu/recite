use std::io::Write;

use recite_compiler::{BuildControl, BuildResultFailure, BuildTerminalStatus, PublishOutcome};
use recite_config::discover_project;

use super::events::WatchState;
use super::{
    ProjectBuildEngine, ProjectBuildPreparation, ProjectBuildPreparationError,
    ProjectBuildPublisher,
};
use crate::diagnostics::report_diagnostics;
use crate::error::CliError;
use crate::i18n::Messages;

#[cfg(test)]
mod tests;

/// The presentation boundary's compact view of one coordinated build.
#[derive(Debug, Eq, PartialEq)]
pub(super) enum BuildStatus {
    Fresh {
        asset_count: usize,
    },
    Stale {
        asset_count: usize,
    },
    Diagnostics,
    PublicationFailure {
        status: BuildTerminalStatus,
        failure: Option<BuildResultFailure>,
        outcome: PublishOutcome,
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
    let result = state
        .coordinator
        .run(
            request.build_request().clone(),
            control,
            &mut engine,
            &mut publisher,
        )
        .map_err(|error| CliError::Watch {
            message: error.to_string(),
        })?;

    report_diagnostics(stderr, messages, result.diagnostics().iter())?;
    if result.status() == BuildTerminalStatus::Succeeded {
        post_publish();
        let freshness = super::freshness::assess_current_freshness(&request)?;
        report_diagnostics(stderr, messages, freshness.diagnostics.iter())?;
        if freshness.stale {
            return Ok(BuildStatus::Stale {
                asset_count: result.candidates().len(),
            });
        }
        if !freshness.diagnostics.is_empty() {
            return Ok(BuildStatus::Diagnostics);
        }
        return Ok(BuildStatus::Fresh {
            asset_count: result.candidates().len(),
        });
    }
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
        return Ok(BuildStatus::Diagnostics);
    }

    Ok(BuildStatus::PublicationFailure {
        status: result.status(),
        failure: result.failure().cloned(),
        outcome: result.publish().clone(),
    })
}

pub(super) fn format_failure(
    status: BuildTerminalStatus,
    failure: Option<&BuildResultFailure>,
    outcome: &PublishOutcome,
) -> String {
    let detail = match outcome {
        PublishOutcome::Partial {
            failed, recovery, ..
        } => format!(
            "partial publication; failed target {failed}; recovery targets: {}",
            format_targets(recovery.targets())
        ),
        PublishOutcome::Indeterminate { recovery, .. } => format!(
            "publication indeterminate; recovery targets: {}",
            format_targets(recovery.targets())
        ),
        PublishOutcome::Refused { reason } => format!("publication refused: {reason:?}"),
        PublishOutcome::NotAttempted { reason } => {
            format!("publication not attempted: {reason:?}")
        }
        PublishOutcome::Published { .. } => "publication reported success".to_owned(),
        _ => "publication returned an unsupported outcome".to_owned(),
    };
    failure.map_or_else(
        || format!("build {status}: {detail}"),
        |failure| format!("{failure}; {detail}"),
    )
}

fn format_targets(targets: &[recite_compiler::BuildTarget]) -> String {
    if targets.is_empty() {
        return "<none>".to_owned();
    }
    targets
        .iter()
        .map(ToString::to_string)
        .collect::<Vec<_>>()
        .join(", ")
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
