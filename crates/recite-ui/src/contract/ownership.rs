use super::{Client, MsgId};

// Keep each client registry in its own domain-owned table while this module
// retains the one exhaustive dispatch point used by UiContract.
include!("ownership/cli.rs");
include!("ownership/tui.rs");
include!("ownership/lsp.rs");
include!("ownership/neovim.rs");

pub(super) const fn clients(id: MsgId) -> &'static [Client] {
    match id {
        tui_message_ids!() => &[Client::Tui],
        MsgId::LspClientDisplayName | MsgId::LspClientRestartExhausted => &[
            Client::Lsp,
            Client::VsCode,
            Client::VsCodium,
            Client::Neovim,
        ],
        neovim_message_ids!() => &[Client::Neovim],
        lsp_client_message_ids!() => &[Client::Lsp, Client::VsCode, Client::VsCodium],
        lsp_message_ids!() => &[Client::Lsp],
        cli_message_ids!() => &[Client::Cli],
    }
}
