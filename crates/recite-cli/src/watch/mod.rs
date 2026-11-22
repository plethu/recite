use std::io::Write;
use std::sync::mpsc;
use std::time::Duration;

use notify::{RecursiveMode, Watcher, recommended_watcher};
use recite_config::discover_project;
use recite_ui::UiArg;
use time::{OffsetDateTime, UtcOffset};

use crate::args::WatchArgs;
use crate::error::CliError;
use crate::fs::display_path;
use crate::i18n::{Messages, MsgId};
use crate::ink::Ink;

mod build;
mod control;
mod emitter;
mod events;
mod freshness;
mod inputs;
mod marker_encoding;
mod protocol;
mod wire;
mod wire_types;

pub use recite_build::{ProjectBuildEngine, ProjectBuildPublisher, ProjectPreparedBuild};
pub use recite_build::{
    ProjectBuildPreparation, ProjectBuildPreparationError, ProjectBuildRequest, ProjectBuildTarget,
};
pub use recite_build::{
    ProjectBuildPublisherError, ProjectBuildRecovery, ProjectBuildRecoveryDetail,
    ProjectBuildRecoveryIoKind, ProjectBuildRecoveryReason,
};
pub use recite_build::{TargetMapError, TargetPathError};

use build::{
    BuildStatus, build_once, format_failure_with_recovery, format_recovery_notice,
    format_recovery_required,
};
use events::{WatchState, drain_debounce, watch_error};

#[cfg(test)]
pub(crate) mod tests;

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
    // Read the local offset before the watcher starts its threads; afterwards it is unavailable.
    let mut status = StatusLine::new(Ink::stderr());

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
        status.ink.teal(messages.format(
            MsgId::WatchBuilding,
            [("path", relative_to_cwd(&state.project_root))]
        ))
    )?;
    let result = build_once(&mut state, stderr, messages);
    status.report(stderr, result, messages)?;

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
        let result = build_once(&mut state, stderr, messages);
        status.report(stderr, result, messages)?;
    }
}

/// The project path as typed relative to the working directory, or `.` when it is the directory.
fn relative_to_cwd(path: &std::path::Path) -> String {
    let relative = std::env::current_dir().ok().and_then(|cwd| {
        path.strip_prefix(cwd)
            .ok()
            .map(std::path::Path::to_path_buf)
    });
    match relative {
        Some(relative) if relative.as_os_str().is_empty() => ".".to_owned(),
        Some(relative) => display_path(&relative),
        None => display_path(path),
    }
}

/// One line per build: local time, a status word and what happened, so a scrolled terminal still
/// shows when the last build ran and whether it recovered from a failure.
struct StatusLine {
    ink: Ink,
    offset: UtcOffset,
    failed: bool,
}

#[derive(Clone, Copy)]
enum Outcome {
    Built,
    Failed,
}

impl StatusLine {
    fn new(ink: Ink) -> Self {
        Self {
            ink,
            offset: UtcOffset::current_local_offset().unwrap_or(UtcOffset::UTC),
            failed: false,
        }
    }

    fn report(
        &mut self,
        stderr: &mut dyn Write,
        result: Result<BuildStatus, CliError>,
        messages: &Messages,
    ) -> Result<(), CliError> {
        let (outcome, detail, duration) = describe_build(result, messages);
        let word = match (outcome, self.failed) {
            (Outcome::Built, true) => self
                .ink
                .teal_strong(pad(messages.text(MsgId::WatchStatusFixed))),
            (Outcome::Built, false) => self
                .ink
                .teal_strong(pad(messages.text(MsgId::WatchStatusOk))),
            (Outcome::Failed, _) => self.ink.error(pad(messages.text(MsgId::WatchStatusError))),
        };
        self.failed = matches!(outcome, Outcome::Failed);
        let duration = duration
            .map(|duration| format!(" · {}", format_duration(messages, duration)))
            .unwrap_or_default();
        writeln!(
            stderr,
            "{}  {word}  {detail}{duration}",
            self.ink.teal(self.clock())
        )?;
        Ok(())
    }

    fn clock(&self) -> String {
        let now = OffsetDateTime::now_utc().to_offset(self.offset);
        format!("{:02}:{:02}:{:02}", now.hour(), now.minute(), now.second())
    }
}

/// Status words share one column; `error` is the longest English word.
fn pad(word: String) -> String {
    format!("{word:<5}")
}

pub(crate) fn run_structured_watch_command(
    args: WatchArgs,
    stdout: &mut dyn Write,
) -> Result<std::process::ExitCode, CliError> {
    protocol::run(args, stdout)
}

fn describe_build(
    result: Result<BuildStatus, CliError>,
    messages: &Messages,
) -> (Outcome, String, Option<Duration>) {
    let status = match result {
        Ok(status) => status,
        Err(CliError::WatchCoordinator { source, recovery }) => {
            return (
                Outcome::Failed,
                recovery_error(messages, source.to_string(), &recovery),
                None,
            );
        }
        Err(CliError::WatchRecovery { source, recovery }) => {
            return (
                Outcome::Failed,
                recovery_error(messages, source.to_user_message(messages), &recovery),
                None,
            );
        }
        Err(error) => {
            return (
                Outcome::Failed,
                messages.format(MsgId::WatchBuildFailed, [("error", error.to_string())]),
                None,
            );
        }
    };
    let duration = status.telemetry().duration();
    let failed = messages.text(MsgId::WatchBuildFailedWaiting);
    let (outcome, detail) = match status {
        BuildStatus::Fresh { asset_count, .. } => (
            Outcome::Built,
            messages.format(MsgId::WatchBuildSucceeded, [("count", asset_count)]),
        ),
        BuildStatus::Stale { recovery, .. }
        | BuildStatus::DiagnosticsWithRecovery { recovery, .. } => {
            (Outcome::Failed, with_recovery(failed, messages, &recovery))
        }
        BuildStatus::Diagnostics { .. } => (Outcome::Failed, failed),
        BuildStatus::RecoveryRequired {
            asset_count,
            recovery,
            ..
        } => (
            Outcome::Failed,
            format_recovery_required(messages, asset_count, &recovery),
        ),
        BuildStatus::PublicationFailure {
            status,
            failure,
            outcome,
            recovery,
            ..
        } => (
            Outcome::Failed,
            format_failure_with_recovery(messages, status, failure.as_ref(), &outcome, &recovery),
        ),
    };
    (outcome, detail, duration)
}

fn with_recovery(detail: String, messages: &Messages, recovery: &[ProjectBuildRecovery]) -> String {
    if recovery.is_empty() {
        detail
    } else {
        format!("{detail}\n{}", format_recovery_notice(messages, recovery))
    }
}

fn format_duration(messages: &Messages, duration: Duration) -> String {
    let (message, value) = if duration < Duration::from_millis(1) {
        (
            MsgId::WatchBuildDurationMicroseconds,
            UiArg::Integer(i64::try_from(duration.as_micros()).unwrap_or(i64::MAX)),
        )
    } else {
        // A tenth of a millisecond matters only for the quickest builds.
        let millis = duration.as_secs_f64() * 1_000.0;
        let shown = if millis < 10.0 {
            (millis * 10.0).round() / 10.0
        } else {
            millis.round()
        };
        (MsgId::WatchBuildDurationMilliseconds, UiArg::Float(shown))
    };
    messages.format(message, [("duration", value)])
}

fn recovery_error(messages: &Messages, error: String, recovery: &[ProjectBuildRecovery]) -> String {
    with_recovery(
        messages.format(MsgId::WatchBuildFailed, [("error", error)]),
        messages,
        recovery,
    )
}
