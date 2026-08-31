use std::io::Write;
use std::sync::mpsc;

use notify::{RecursiveMode, Watcher, recommended_watcher};
use recite_config::discover_project;

use crate::args::WatchArgs;
use crate::error::CliError;
use crate::fs::display_path;
use crate::i18n::{Messages, MsgId};

mod build;
mod commit;
mod engine;
mod events;
mod freshness;
mod inputs;
mod preparation;
mod publisher;
mod recovery;
mod request;
mod staging;
mod target_identity;
mod targets;

pub use engine::ProjectBuildEngine;
pub use publisher::{ProjectBuildPublisher, ProjectPreparedBuild};
pub use recovery::{ProjectBuildPublisherError, ProjectBuildRecovery};
pub use request::{
    ProjectBuildPreparation, ProjectBuildPreparationError, ProjectBuildRequest, ProjectBuildTarget,
};
pub use targets::{TargetMapError, TargetPathError};

use build::{BuildStatus, build_once, format_failure};
use events::{WatchState, drain_debounce, watch_error};

#[cfg(test)]
mod tests;

pub(super) const PROJECT_MANIFEST_FILE: &str = "recite.project.toml";

pub(crate) fn run_watch_command(
    args: WatchArgs,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<(), CliError> {
    if !args.project_root.is_dir() {
        return Err(CliError::MissingPath(args.project_root));
    }
    let discovery = discover_project(&args.project_root)
        .map_err(|source| CliError::ProjectDiscovery { source })?;
    let project_root = discovery.manifest().project_root().to_owned();

    let (sender, receiver) = mpsc::channel();
    let mut watcher = recommended_watcher(move |event| {
        let _ = sender.send(event);
    })
    .map_err(watch_error)?;
    watcher
        .watch(&project_root, RecursiveMode::Recursive)
        .map_err(watch_error)?;

    let mut state = WatchState::new(project_root);
    state.manifest = Some(discovery.manifest().clone());
    writeln!(
        stderr,
        "{}",
        messages.format(
            MsgId::WatchBuilding,
            [("path", display_path(&state.project_root))]
        )
    )?;
    let result = build_once(&mut state, stderr, messages);
    report_build_result(stderr, result, messages)?;
    writeln!(stderr, "{}", messages.text(MsgId::WatchWaitingForChanges))?;

    loop {
        let event = receiver.recv().map_err(|_| CliError::Watch {
            message: "watcher event channel closed".to_owned(),
        })?;
        let event = match event {
            Ok(event) => event,
            Err(error) => {
                writeln!(
                    stderr,
                    "{}",
                    messages.format(MsgId::WatchEventError, [("error", error.to_string())])
                )?;
                continue;
            }
        };

        if !state.is_relevant_event(&event) {
            continue;
        }

        drain_debounce(&receiver, &state, stderr, messages)?;
        writeln!(stderr, "{}", messages.text(MsgId::WatchRebuilding))?;
        let result = build_once(&mut state, stderr, messages);
        report_build_result(stderr, result, messages)?;
    }
}

fn report_build_result(
    stderr: &mut dyn Write,
    result: Result<BuildStatus, CliError>,
    messages: &Messages,
) -> Result<(), CliError> {
    match result {
        Ok(BuildStatus::Fresh { asset_count }) => {
            writeln!(
                stderr,
                "{}",
                messages.format(MsgId::WatchBuildSucceeded, [("count", asset_count)])
            )?;
        }
        Ok(BuildStatus::Stale { .. }) => {
            writeln!(stderr, "{}", messages.text(MsgId::WatchBuildFailedWaiting))?;
        }
        Ok(BuildStatus::Diagnostics) => {
            writeln!(stderr, "{}", messages.text(MsgId::WatchBuildFailedWaiting))?;
        }
        Ok(BuildStatus::PublicationFailure {
            status,
            failure,
            outcome,
        }) => {
            let error = format_failure(status, failure.as_ref(), &outcome);
            writeln!(
                stderr,
                "{}",
                messages.format(MsgId::WatchBuildFailed, [("error", error)])
            )?;
        }
        Err(error) => {
            writeln!(
                stderr,
                "{}",
                messages.format(MsgId::WatchBuildFailed, [("error", error.to_string())])
            )?;
        }
    }
    Ok(())
}
