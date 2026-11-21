//! Help text for `recite prose` and `recite prose check`.

use super::{localise_group, localise_paths, set_about};
use crate::i18n::{Messages, MsgId};

pub(super) fn localise(command: &mut clap::Command, messages: &Messages, path: &str) {
    if path == "prose" {
        localise_group(command, messages, MsgId::CliHelpCommandProse, path);
    } else {
        set_about(command, messages.text(MsgId::CliHelpCommandProseCheck));
        localise_paths(command, messages);
    }
}
