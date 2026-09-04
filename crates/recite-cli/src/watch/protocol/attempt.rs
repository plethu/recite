use std::io::{self, Write};
use std::sync::mpsc::Receiver;

use recite_compiler::{BuildControl, BuildStatusProjection};

use crate::error::CliError;
use crate::structured::error_mapping::structured_error;

use super::super::build::build_once_with_control;
use super::super::control::{ControlMessage, ControlTransport};
use super::super::emitter::WatchProtocol;
use super::super::events::WatchState;
use super::super::wire_types::{BuildCompletedData, BuildStartedData, BuildTriggerDto};
use super::acknowledge_cancel;

pub(super) struct WatchAttempt {
    data: BuildCompletedData,
}

pub(super) fn run_attempt(
    state: &mut WatchState,
    transport: &ControlTransport,
    protocol: &mut WatchProtocol<'_>,
    trigger: BuildTriggerDto,
) -> Result<WatchAttempt, CliError> {
    run_attempt_with(
        state,
        transport,
        protocol,
        trigger,
        |state, sink, control| {
            build_once_with_control(state, sink, &crate::default_messages(), control)
        },
    )
}

pub(super) fn run_attempt_with<F>(
    state: &mut WatchState,
    transport: &ControlTransport,
    protocol: &mut WatchProtocol<'_>,
    trigger: BuildTriggerDto,
    build: F,
) -> Result<WatchAttempt, CliError>
where
    F: FnOnce(
        &mut WatchState,
        &mut dyn Write,
        &BuildControl,
    ) -> Result<super::super::build::BuildStatus, CliError>,
{
    let generation = state.next_build_generation_number();
    protocol.build_started(BuildStartedData {
        generation,
        trigger,
    })?;
    let control = BuildControl::new();
    transport.begin_build(&control);
    let mut sink = io::sink();
    let result = build(state, &mut sink, &control);
    transport.end_build();

    match result {
        Ok(status) => {
            let projection = BuildStatusProjection::from_state(state.coordinator.state());
            let data = if projection.generation() == state.last_build_generation() {
                BuildCompletedData::from_projection(
                    &projection,
                    &state.project_root,
                    &status,
                    status.recovery(),
                )
            } else {
                BuildCompletedData::from_diagnostics(
                    generation,
                    state.preparation_inputs(),
                    state.preparation_diagnostics(),
                )
            };
            match data {
                Ok(data) => Ok(WatchAttempt { data }),
                Err(error) => {
                    let mapped = structured_error(&error, "build", Some(&state.project_root));
                    Ok(WatchAttempt {
                        data: BuildCompletedData::from_error(
                            generation,
                            state.preparation_inputs(),
                            mapped,
                        ),
                    })
                }
            }
        }
        Err(error) => {
            let mapped = structured_error(&error, "build", Some(&state.project_root));
            let projection = BuildStatusProjection::from_state(state.coordinator.state());
            let recovery = error_recovery(&error);
            if projection.generation() == state.last_build_generation()
                && let Ok(data) = BuildCompletedData::from_projection_error(
                    &projection,
                    &state.project_root,
                    recovery,
                    mapped,
                )
            {
                return Ok(WatchAttempt { data });
            }
            Ok(WatchAttempt {
                data: BuildCompletedData::from_error(
                    generation,
                    state.preparation_inputs(),
                    structured_error(&error, "build", Some(&state.project_root)),
                ),
            })
        }
    }
}

pub(super) fn complete_attempt(
    attempt: WatchAttempt,
    transport: &ControlTransport,
    receiver: &Receiver<ControlMessage>,
    control_open: &mut bool,
    protocol: &mut WatchProtocol<'_>,
    cancel_emitted: &mut bool,
) -> Result<bool, CliError> {
    let cancellation =
        acknowledge_cancel(transport, receiver, control_open, protocol, cancel_emitted);
    protocol.build_completed(attempt.data)?;
    cancellation
}

fn error_recovery(error: &CliError) -> &[super::super::ProjectBuildRecovery] {
    match error {
        CliError::WatchCoordinator { recovery, .. } | CliError::WatchRecovery { recovery, .. } => {
            recovery
        }
        _ => &[],
    }
}
