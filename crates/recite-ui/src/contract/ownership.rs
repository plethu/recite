use super::{Client, MsgId};

// Keep each client registry in its own domain-owned table while this module
// retains the one exhaustive dispatch point used by UiContract.
include!("ownership/cli.rs");
include!("ownership/tui.rs");
include!("ownership/lsp.rs");

pub(super) const fn clients(id: MsgId) -> &'static [Client] {
    match id {
        tui_message_ids!() => &[Client::Tui],
        lsp_message_ids!() => &[Client::Lsp],
        cli_message_ids!() => &[Client::Cli],
    }
}
