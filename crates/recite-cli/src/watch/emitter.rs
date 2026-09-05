use std::io::Write;

use serde::Serialize;

use crate::error::CliError;
use crate::schema_inspection::{MachinePathProjection, machine_path};
use crate::structured::errors::StructuredError;

use super::control::ControlError;
use super::wire_types::{BuildCompletedData, BuildStartedData, CancellationDto};

pub(super) struct WatchProtocol<'a> {
    output: &'a mut dyn Write,
    invocation_id: Option<String>,
    sequence: u64,
}

impl<'a> WatchProtocol<'a> {
    pub(super) fn new(output: &'a mut dyn Write, invocation_id: Option<String>) -> Self {
        Self {
            output,
            invocation_id,
            sequence: 0,
        }
    }

    pub(super) fn started(&mut self, project_root: &std::path::Path) -> Result<(), CliError> {
        self.write(
            "watch.started",
            StartedData {
                project_root: machine_path(project_root),
            },
        )
    }

    pub(super) fn build_started(&mut self, data: BuildStartedData) -> Result<(), CliError> {
        self.write("watch.build.started", data)
    }

    pub(super) fn build_completed(&mut self, data: BuildCompletedData) -> Result<(), CliError> {
        self.write("watch.build.completed", data)
    }

    pub(super) fn waiting(&mut self) -> Result<(), CliError> {
        self.write("watch.waiting", EmptyData {})
    }

    pub(super) fn cancel_requested(
        &mut self,
        cancellation: CancellationDto,
    ) -> Result<(), CliError> {
        self.write(
            "watch.cancel.requested",
            CancelRequestedData { cancellation },
        )
    }

    pub(super) fn control_error(&mut self, error: ControlError) -> Result<(), CliError> {
        self.write(
            "watch.control.error",
            ControlErrorData {
                error: ControlErrorDto::from(error),
            },
        )
    }

    pub(super) fn notify_error(&mut self) -> Result<(), CliError> {
        self.write(
            "watch.notify.error",
            NotifyErrorData {
                error: NotifyErrorDto::Watcher,
            },
        )
    }

    pub(super) fn stopped(
        &mut self,
        reason: StopReasonDto,
        error: Option<StructuredError>,
    ) -> Result<(), CliError> {
        self.write("watch.stopped", StoppedData { reason, error })
    }

    fn write<T: Serialize>(&mut self, event: &'static str, data: T) -> Result<(), CliError> {
        serde_json::to_writer(
            &mut *self.output,
            &WatchRecord {
                version: 1,
                sequence: self.sequence,
                event,
                command: "watch",
                invocation_id: self.invocation_id.clone(),
                data,
            },
        )
        .map_err(CliError::TraceJson)?;
        self.output.write_all(b"\n")?;
        self.output.flush()?;
        self.sequence += 1;
        Ok(())
    }
}

#[derive(Serialize)]
struct WatchRecord<T> {
    version: u16,
    sequence: u64,
    event: &'static str,
    command: &'static str,
    #[serde(skip_serializing_if = "Option::is_none")]
    invocation_id: Option<String>,
    data: T,
}

#[derive(Serialize)]
struct StartedData {
    project_root: MachinePathProjection,
}

#[derive(Serialize)]
struct EmptyData {}

#[derive(Serialize)]
struct CancelRequestedData {
    cancellation: CancellationDto,
}

#[derive(Serialize)]
struct ControlErrorData {
    error: ControlErrorDto,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum ControlErrorDto {
    Malformed,
    UnsupportedVersion,
    UnsupportedCommand,
    UnsupportedAction,
    InvocationMismatch,
}

impl From<ControlError> for ControlErrorDto {
    fn from(error: ControlError) -> Self {
        match error {
            ControlError::Malformed => Self::Malformed,
            ControlError::UnsupportedVersion => Self::UnsupportedVersion,
            ControlError::UnsupportedCommand => Self::UnsupportedCommand,
            ControlError::UnsupportedAction => Self::UnsupportedAction,
            ControlError::InvocationMismatch => Self::InvocationMismatch,
        }
    }
}

#[derive(Serialize)]
struct NotifyErrorData {
    error: NotifyErrorDto,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
enum NotifyErrorDto {
    Watcher,
}

#[derive(Serialize)]
#[serde(tag = "type", rename_all = "snake_case")]
pub(super) enum StopReasonDto {
    Cancelled,
    Fatal,
}

#[derive(Serialize)]
struct StoppedData {
    reason: StopReasonDto,
    #[serde(skip_serializing_if = "Option::is_none")]
    error: Option<StructuredError>,
}
