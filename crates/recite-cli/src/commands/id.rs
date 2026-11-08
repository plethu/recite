//! `id check`, `id assign`, `id rename` and `id reassign`: stable line and
//! choice IDs.

use std::io::Write;

use crate::args::IdCommand;
use crate::error::CliError;
use crate::i18n::Messages;

mod assign;
mod check;
mod rename;

pub(super) fn run(
    command: IdCommand,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<(), CliError> {
    match command {
        IdCommand::Check(args) => check::run(&args, stderr, messages),
        IdCommand::Assign(args) => assign::run(args, stderr, messages),
        IdCommand::Rename(args) => rename::run(
            &args.target,
            rename::NewId::Named(&args.new_id),
            &args.project,
            stderr,
            messages,
        ),
        IdCommand::Reassign(args) => rename::run(
            &args.target,
            rename::NewId::Generated,
            &args.project,
            stderr,
            messages,
        ),
    }
}
