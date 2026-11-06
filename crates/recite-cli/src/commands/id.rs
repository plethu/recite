//! `id check` and `id assign`: stable line and choice IDs.

use std::io::Write;

use crate::args::IdCommand;
use crate::error::CliError;
use crate::i18n::Messages;

mod assign;
mod check;

pub(super) fn run(
    command: IdCommand,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<(), CliError> {
    match command {
        IdCommand::Check(args) => check::run(&args, stderr, messages),
        IdCommand::Assign(args) => assign::run(args, stderr, messages),
    }
}
