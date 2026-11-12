//! Help text for `recite recording status`, `export` and `import`.

use super::{set_about, set_arg_help, set_positional_arg_help};
use crate::i18n::{Messages, MsgId};

pub(super) fn localise(command: &mut clap::Command, messages: &Messages, path: &str) {
    let about = match path {
        "recording status" => MsgId::CliHelpCommandRecordingStatus,
        "recording export" => MsgId::CliHelpCommandRecordingExport,
        _ => MsgId::CliHelpCommandRecordingImport,
    };
    set_about(command, messages.text(about));
    // `mut_arg` moves an argument to the end, so positionals are localised in usage order.
    if path == "recording import" {
        set_positional_arg_help(
            command,
            "file",
            messages.text(MsgId::CliHelpArgRecordingFile),
            messages,
        );
    }
    set_positional_arg_help(
        command,
        "project_root",
        messages.text(MsgId::CliHelpArgProjectRoot),
        messages,
    );
    set_arg_help(
        command,
        "locale",
        messages.text(MsgId::CliHelpArgRecordingLocale),
        messages,
    );
    let options: &[(&'static str, MsgId)] = match path {
        "recording status" => &[
            ("check", MsgId::CliHelpArgRecordingCheck),
            ("format", MsgId::CliHelpArgCatalogFormat),
        ],
        "recording export" => &[
            ("blocks", MsgId::CliHelpArgRecordingBlock),
            ("states", MsgId::CliHelpArgRecordingState),
            ("handoff", MsgId::CliHelpArgRecordingHandoff),
            ("format", MsgId::CliHelpArgRecordingFormat),
            ("output", MsgId::CliHelpArgRecordingOutput),
        ],
        _ => &[
            ("format", MsgId::CliHelpArgRecordingFormat),
            ("apply", MsgId::CliHelpArgRecordingApply),
        ],
    };
    for (id, help) in options {
        set_arg_help(command, id, messages.text(*help), messages);
    }
}
