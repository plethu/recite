//! Help text for `recite save`, `recite save impact` and `recite save inspect`.

use super::{localise_group, set_about, set_arg_help, set_positional_arg_help};
use crate::i18n::{Messages, MsgId};

pub(super) fn localise(command: &mut clap::Command, messages: &Messages, path: &str) {
    match path {
        "save impact" => {
            set_about(command, messages.text(MsgId::CliHelpCommandSaveImpact));
            set_positional_arg_help(
                command,
                "candidate",
                messages.text(MsgId::CliHelpArgSaveCandidate),
                messages,
            );
            for (id, message) in [
                ("baseline", MsgId::CliHelpArgSaveBaseline),
                ("since", MsgId::CliHelpArgSaveSince),
                ("check", MsgId::CliHelpArgSaveCheck),
                ("allow_fallback", MsgId::CliHelpArgAllowFallback),
                ("format", MsgId::CliHelpArgSaveFormat),
            ] {
                set_arg_help(command, id, messages.text(message), messages);
            }
        }
        "save inspect" => {
            set_about(command, messages.text(MsgId::CliHelpCommandSaveInspect));
            set_positional_arg_help(
                command,
                "save",
                messages.text(MsgId::CliHelpArgSaveFile),
                messages,
            );
            for (id, message) in [
                ("asset", MsgId::CliHelpArgSaveAsset),
                ("format", MsgId::CliHelpArgSaveFormat),
            ] {
                set_arg_help(command, id, messages.text(message), messages);
            }
        }
        _ => localise_group(command, messages, MsgId::CliHelpCommandSave, path),
    }
}
