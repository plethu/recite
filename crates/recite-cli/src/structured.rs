//! Versioned, command-local machine output for the non-interactive CLI.
//!
//! This is deliberately an adapter at the CLI boundary. Compiler diagnostics,
//! runtime traces, and path projections keep their owning representations;
//! this module only assembles the finite process protocol around them.

use std::io::Write;
use std::process::ExitCode;

use crate::args::Command;
use crate::error::CliError;

#[path = "structured/data.rs"]
pub(crate) mod data;
mod emitter;
#[path = "structured/error_mapping.rs"]
pub(crate) mod error_mapping;
#[path = "structured/errors.rs"]
pub(crate) mod errors;
mod operations;

use emitter::ProtocolWriter;
use error_mapping::structured_error;

const PROTOCOL_VERSION: u16 = 1;

pub(crate) fn run(command: Command, stdout: &mut dyn Write) -> Result<ExitCode, CliError> {
    let command = match command {
        Command::Watch(args) => return crate::watch::run_structured_watch_command(args, stdout),
        command => command,
    };

    let Some(invocation) = command.structured_invocation() else {
        return Err(CliError::MalformedCompiledAsset {
            reason: "structured protocol was requested for an unsupported command".to_owned(),
        });
    };

    let mut protocol = ProtocolWriter::new(stdout);
    protocol.started(&invocation)?;

    match operations::execute(command) {
        Ok(outcome) => {
            let exit_code = outcome.exit_code();
            protocol.result(&invocation, outcome)?;
            Ok(ExitCode::from(exit_code))
        }
        Err(failure) => {
            protocol.error(
                &invocation,
                structured_error(&failure.error, failure.operation, failure.path.as_deref()),
            )?;
            Ok(ExitCode::from(1))
        }
    }
}
