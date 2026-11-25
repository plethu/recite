//! Shared help text for arguments that several commands declare.
use crate::i18n::{Messages, MsgId};

/// Commands whose paths and schema default to the project's (§13).
pub(super) fn localise_project_defaults(command: &mut clap::Command, messages: &Messages) {
    set_arg_help(
        command,
        "paths",
        messages.text(MsgId::CliHelpArgPathsProject),
        messages,
    );
    set_arg_help(
        command,
        "schema",
        messages.text(MsgId::CliHelpArgSchemaProject),
        messages,
    );
}

pub(super) fn localise_paths(command: &mut clap::Command, messages: &Messages) {
    set_arg_help(
        command,
        "paths",
        messages.text(MsgId::CliHelpArgPaths),
        messages,
    );
}

pub(super) fn localise_schema(command: &mut clap::Command, messages: &Messages) {
    set_arg_help(
        command,
        "schema",
        messages.text(MsgId::CliHelpArgSchema),
        messages,
    );
}

pub(super) fn localise_runtime_args(
    command: &mut clap::Command,
    messages: &Messages,
    asset: MsgId,
) {
    set_arg_help(command, "asset", messages.text(asset), messages);
    set_arg_help(
        command,
        "block",
        messages.text(MsgId::CliHelpArgBlockProject),
        messages,
    );
    set_arg_help(
        command,
        "fixture",
        messages.text(MsgId::CliHelpArgFixture),
        messages,
    );
}

pub(super) fn set_arg_help(
    command: &mut clap::Command,
    id: &'static str,
    help: String,
    messages: &Messages,
) {
    // The same id can be positional in one command and an option in another
    // (`run <ASSET>` and `save inspect --asset`), so the argument decides.
    let arguments = messages.text(MsgId::CliHelpArgumentsHeading);
    let options = messages.text(MsgId::CliHelpOptionsHeading);
    *command = std::mem::take(command).mut_arg(id, |arg| {
        let heading = if arg.is_positional() {
            arguments
        } else {
            options
        };
        arg.help(help).help_heading(heading)
    });
}

pub(super) fn set_positional_arg_help(
    command: &mut clap::Command,
    id: &'static str,
    help: String,
    messages: &Messages,
) {
    *command = std::mem::take(command).mut_arg(id, |arg| {
        arg.help(help)
            .help_heading(messages.text(MsgId::CliHelpArgumentsHeading))
    });
}

pub(super) fn set_about(command: &mut clap::Command, about: String) {
    *command = std::mem::take(command).about(about);
}
