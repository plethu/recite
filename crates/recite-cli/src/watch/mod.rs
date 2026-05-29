use std::fs;
use std::io::Write;
use std::sync::mpsc;

use notify::{RecursiveMode, Watcher, recommended_watcher};

use crate::args::WatchArgs;
use crate::error::CliError;
use crate::fs::display_path;

mod build;
mod events;
mod inputs;

use build::{BuildStatus, build_once};
use events::{WatchState, drain_debounce, watch_error};

#[cfg(test)]
mod tests;

pub(super) const PROJECT_MANIFEST_FILE: &str = "recite.project.toml";

pub(crate) fn run_watch_command(args: WatchArgs, stderr: &mut dyn Write) -> Result<(), CliError> {
    if !args.project_root.is_dir() {
        return Err(CliError::MissingPath(args.project_root));
    }
    let project_root =
        fs::canonicalize(&args.project_root).map_err(|source| CliError::ReadDir {
            path: args.project_root,
            source,
        })?;

    let (sender, receiver) = mpsc::channel();
    let mut watcher = recommended_watcher(move |event| {
        let _ = sender.send(event);
    })
    .map_err(watch_error)?;
    watcher
        .watch(&project_root, RecursiveMode::Recursive)
        .map_err(watch_error)?;

    let mut state = WatchState::new(project_root);
    writeln!(
        stderr,
        "watch: building {}",
        display_path(&state.project_root)
    )?;
    let result = build_once(&mut state, stderr);
    report_build_result(stderr, result)?;
    writeln!(stderr, "watch: waiting for changes")?;

    loop {
        let event = receiver.recv().map_err(|_| CliError::Watch {
            message: "watcher event channel closed".to_owned(),
        })?;
        let event = match event {
            Ok(event) => event,
            Err(error) => {
                writeln!(stderr, "watch: watcher event error: {error}")?;
                continue;
            }
        };

        if !state.is_relevant_event(&event) {
            continue;
        }

        drain_debounce(&receiver, &state, stderr)?;
        writeln!(stderr, "watch: rebuilding")?;
        let result = build_once(&mut state, stderr);
        report_build_result(stderr, result)?;
    }
}

fn report_build_result(
    stderr: &mut dyn Write,
    result: Result<BuildStatus, CliError>,
) -> Result<(), CliError> {
    match result {
        Ok(BuildStatus::Fresh { asset_count }) => {
            writeln!(stderr, "watch: build succeeded ({asset_count} assets)")?;
        }
        Ok(BuildStatus::Diagnostics) => {
            writeln!(stderr, "watch: build failed; waiting for changes")?;
        }
        Err(error) => {
            writeln!(stderr, "watch: build failed: {error}")?;
        }
    }
    Ok(())
}
