use std::io::{self, Write};
use std::sync::mpsc::{self, Receiver, TryRecvError};
use std::time::Duration;

use crate::args::WatchArgs;
use crate::error::CliError;
use crate::structured::error_mapping::structured_error;
use notify::{Event, Watcher, recommended_watcher};

use super::control::{ControlMessage, ControlTransport};
use super::emitter::{StopReasonDto, WatchProtocol};
use super::events::{WatchState, monotonic_now, watch_error};
use super::wire_types::{BuildTriggerDto, CancellationDto};

const DEBOUNCE: Duration = Duration::from_millis(250);
const CONTROL_POLL: Duration = Duration::from_millis(25);

#[cfg(test)]
mod tests;

#[path = "protocol/attempt.rs"]
mod attempt;

use attempt::{complete_attempt, run_attempt};

/// Run the version-1 streaming watch protocol. Human watch output is kept in
/// [`super::run_watch_command`]; this function owns only the opt-in transport.
pub(super) fn run(
    args: WatchArgs,
    stdout: &mut dyn Write,
) -> Result<std::process::ExitCode, CliError> {
    let invocation_id = args.invocation_id.clone();
    let mut protocol = WatchProtocol::new(stdout, invocation_id);
    match run_inner(args, &mut protocol) {
        Ok(exit_code) => Ok(exit_code),
        Err(error) => stop_with_error(&mut protocol, error, "watch", None),
    }
}

fn run_inner(
    args: WatchArgs,
    protocol: &mut WatchProtocol<'_>,
) -> Result<std::process::ExitCode, CliError> {
    match std::fs::metadata(&args.project_root) {
        Ok(metadata) if metadata.is_dir() => {}
        Ok(_) => {
            protocol.started(&args.project_root)?;
            let path = args.project_root;
            let error = CliError::InvalidProjectRoot(path.clone());
            return stop_with_error(protocol, error, "resolve_path", Some(&path));
        }
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            protocol.started(&args.project_root)?;
            let path = args.project_root;
            let error = CliError::MissingPath(path.clone());
            return stop_with_error(protocol, error, "resolve_path", Some(&path));
        }
        Err(error) => {
            protocol.started(&args.project_root)?;
            let path = args.project_root;
            return stop_with_error(protocol, CliError::Io(error), "resolve_path", Some(&path));
        }
    }

    let discovery = match recite_config::discover_project(&args.project_root) {
        Ok(discovery) => discovery,
        Err(source) => {
            let path = source
                .manifest_path()
                .map_or_else(|| args.project_root.clone(), std::path::Path::to_owned);
            let error = CliError::ProjectDiscovery { source };
            protocol.started(&args.project_root)?;
            return stop_with_error(protocol, error, "discover_project", Some(&path));
        }
    };
    let project_root = discovery.manifest().project_root().to_owned();
    protocol.started(&project_root)?;

    let (event_sender, event_receiver) = mpsc::channel();
    let mut watcher = match recommended_watcher(move |event| {
        let _ = event_sender.send(event);
    }) {
        Ok(watcher) => watcher,
        Err(error) => {
            return stop_with_error(protocol, watch_error(error), "start_watcher", None);
        }
    };
    if let Err(error) = watcher.watch(&project_root, notify::RecursiveMode::Recursive) {
        return stop_with_error(protocol, watch_error(error), "watch_project", None);
    }

    let (transport, control_receiver) =
        ControlTransport::spawn(io::stdin(), args.invocation_id.as_deref());
    let mut control_open = true;
    let mut state = WatchState::new(project_root.clone());
    state.update_from_discovery(&discovery);
    let mut cancel_emitted = false;

    let initial = run_attempt(&mut state, &transport, protocol, BuildTriggerDto::Initial)?;
    let initial_cancelled = complete_attempt(
        initial,
        &transport,
        &control_receiver,
        &mut control_open,
        protocol,
        &mut cancel_emitted,
    )?;
    if initial_cancelled {
        return stop_cancelled(protocol);
    }
    protocol.waiting()?;

    loop {
        if drain_controls(
            &control_receiver,
            &mut control_open,
            protocol,
            &mut cancel_emitted,
        )? {
            return stop_cancelled(protocol);
        }
        let event = match receive_event(
            &event_receiver,
            &control_receiver,
            &mut control_open,
            protocol,
            &mut cancel_emitted,
        )? {
            Some(event) => event,
            None => return stop_cancelled(protocol),
        };
        let event = match event {
            Ok(event) => event,
            Err(error) => {
                protocol.notify_error()?;
                let _ = error;
                continue;
            }
        };
        if !state.is_relevant_event(&event) {
            continue;
        }

        if drain_structured_debounce(
            &event_receiver,
            &control_receiver,
            &mut control_open,
            &state,
            protocol,
            &mut cancel_emitted,
        )? {
            return stop_cancelled(protocol);
        }
        let attempt = run_attempt(
            &mut state,
            &transport,
            protocol,
            BuildTriggerDto::InputChanged,
        )?;
        let attempt_cancelled = complete_attempt(
            attempt,
            &transport,
            &control_receiver,
            &mut control_open,
            protocol,
            &mut cancel_emitted,
        )?;
        if attempt_cancelled {
            return stop_cancelled(protocol);
        }
        protocol.waiting()?;
    }
}

fn drain_structured_debounce(
    events: &Receiver<notify::Result<Event>>,
    controls: &Receiver<ControlMessage>,
    control_open: &mut bool,
    state: &WatchState,
    protocol: &mut WatchProtocol<'_>,
    cancel_emitted: &mut bool,
) -> Result<bool, CliError> {
    let mut deadline = monotonic_now() + DEBOUNCE;
    loop {
        if drain_controls(controls, control_open, protocol, cancel_emitted)? {
            return Ok(true);
        }
        let now = monotonic_now();
        if now >= deadline {
            return Ok(false);
        }
        let wait = (deadline - now).min(CONTROL_POLL);
        match events.recv_timeout(wait) {
            Ok(Ok(event)) => {
                if state.is_relevant_event(&event) {
                    deadline = monotonic_now() + DEBOUNCE;
                }
            }
            Ok(Err(_error)) => protocol.notify_error()?,
            Err(mpsc::RecvTimeoutError::Timeout) => {}
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(CliError::Watch {
                    message: "watcher event channel closed".to_owned(),
                });
            }
        }
    }
}

fn receive_event(
    events: &Receiver<notify::Result<Event>>,
    controls: &Receiver<ControlMessage>,
    control_open: &mut bool,
    protocol: &mut WatchProtocol<'_>,
    cancel_emitted: &mut bool,
) -> Result<Option<notify::Result<Event>>, CliError> {
    loop {
        if drain_controls(controls, control_open, protocol, cancel_emitted)? {
            return Ok(None);
        }
        let result = if *control_open {
            events.recv_timeout(CONTROL_POLL)
        } else {
            events
                .recv()
                .map_err(|_| mpsc::RecvTimeoutError::Disconnected)
        };
        match result {
            Ok(event) => return Ok(Some(event)),
            Err(mpsc::RecvTimeoutError::Timeout) => continue,
            Err(mpsc::RecvTimeoutError::Disconnected) => {
                return Err(CliError::Watch {
                    message: "watcher event channel closed".to_owned(),
                });
            }
        }
    }
}

fn drain_controls(
    receiver: &Receiver<ControlMessage>,
    control_open: &mut bool,
    protocol: &mut WatchProtocol<'_>,
    cancel_emitted: &mut bool,
) -> Result<bool, CliError> {
    if !*control_open {
        return Ok(false);
    }
    loop {
        match receiver.try_recv() {
            Ok(ControlMessage::Cancel) => {
                if !*cancel_emitted {
                    protocol.cancel_requested(CancellationDto::User)?;
                    *cancel_emitted = true;
                }
                return Ok(true);
            }
            Ok(ControlMessage::Error(error)) => protocol.control_error(error)?,
            Ok(ControlMessage::Stream(error)) => return Err(CliError::Io(error)),
            Err(TryRecvError::Empty) => return Ok(false),
            Err(TryRecvError::Disconnected) => {
                *control_open = false;
                return Ok(false);
            }
        }
    }
}

fn acknowledge_cancel(
    transport: &ControlTransport,
    receiver: &Receiver<ControlMessage>,
    control_open: &mut bool,
    protocol: &mut WatchProtocol<'_>,
    cancel_emitted: &mut bool,
) -> Result<bool, CliError> {
    let received = drain_controls(receiver, control_open, protocol, cancel_emitted)?;
    if received || transport.requested() {
        if !*cancel_emitted {
            protocol.cancel_requested(CancellationDto::User)?;
            *cancel_emitted = true;
        }
        Ok(true)
    } else {
        Ok(false)
    }
}

fn stop_cancelled(protocol: &mut WatchProtocol<'_>) -> Result<std::process::ExitCode, CliError> {
    protocol.stopped(StopReasonDto::Cancelled, None)?;
    Ok(std::process::ExitCode::SUCCESS)
}

fn stop_with_error(
    protocol: &mut WatchProtocol<'_>,
    error: CliError,
    operation: &'static str,
    path: Option<&std::path::Path>,
) -> Result<std::process::ExitCode, CliError> {
    let mapped = structured_error(&error, operation, path);
    protocol.stopped(StopReasonDto::Fatal, Some(mapped))?;
    Ok(std::process::ExitCode::from(1))
}
