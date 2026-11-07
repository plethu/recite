//! `schema inspect`, `schema export` and `schema check-fresh`.
//!
//! The implementations stay beside the schema projections they own; this
//! module is the single place the command tree reaches them.

use std::io::Write;

use crate::args::SchemaCommand;
use crate::error::CliError;
use crate::i18n::Messages;

pub(super) fn run(
    command: SchemaCommand,
    stdout: &mut dyn Write,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<(), CliError> {
    match command {
        SchemaCommand::Inspect(args) => {
            crate::schema_inspection::run(args, stdout, stderr, messages)
        }
        SchemaCommand::Export(args) => crate::schema_export::run(args, stdout, stderr, messages),
        SchemaCommand::CheckFresh(args) => {
            crate::schema_freshness::check(args, stdout, stderr, messages)
        }
    }
}
