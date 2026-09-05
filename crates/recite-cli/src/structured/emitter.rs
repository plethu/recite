use std::io::Write;

use crate::args::StructuredInvocation;
use crate::error::CliError;

use super::data::{ErrorRecord, ErrorStatus, ResultRecord, StartedRecord, StructuredOutcome};
use super::errors::StructuredError;

// Keep the protocol version in one place while allowing the structured module
// to expose its wire records without exposing implementation details.
use super::PROTOCOL_VERSION;

pub(super) struct ProtocolWriter<'a> {
    output: &'a mut dyn Write,
    sequence: u64,
}

impl<'a> ProtocolWriter<'a> {
    pub(super) fn new(output: &'a mut dyn Write) -> Self {
        Self {
            output,
            sequence: 0,
        }
    }

    pub(super) fn started(&mut self, invocation: &StructuredInvocation) -> Result<(), CliError> {
        self.write_record(&StartedRecord {
            version: PROTOCOL_VERSION,
            sequence: self.sequence,
            event: "command.started",
            command: invocation.command,
            invocation_id: invocation.invocation_id.as_deref(),
        })
    }

    pub(super) fn result(
        &mut self,
        invocation: &StructuredInvocation,
        outcome: StructuredOutcome,
    ) -> Result<(), CliError> {
        let (status, data) = outcome.into_parts();
        let exit_code = status.exit_code();
        self.write_record(&ResultRecord {
            version: PROTOCOL_VERSION,
            sequence: self.sequence,
            event: "command.result",
            command: invocation.command,
            invocation_id: invocation.invocation_id.clone(),
            status,
            exit_code,
            data,
        })
    }

    pub(super) fn error(
        &mut self,
        invocation: &StructuredInvocation,
        error: StructuredError,
    ) -> Result<(), CliError> {
        self.write_record(&ErrorRecord {
            version: PROTOCOL_VERSION,
            sequence: self.sequence,
            event: "command.error",
            command: invocation.command,
            invocation_id: invocation.invocation_id.clone(),
            status: ErrorStatus::Failure,
            exit_code: 1,
            error,
        })
    }

    fn write_record<T: serde::Serialize>(&mut self, record: &T) -> Result<(), CliError> {
        serde_json::to_writer(&mut *self.output, record).map_err(CliError::TraceJson)?;
        self.output.write_all(b"\n")?;
        self.output.flush()?;
        self.sequence += 1;
        Ok(())
    }
}
