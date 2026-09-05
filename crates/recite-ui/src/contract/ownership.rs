use super::{Client, MsgId};

// Keep each client registry in its own domain-owned table while this module
// retains the one exhaustive dispatch point used by UiContract.
include!("ownership/cli.rs");
include!("ownership/tui.rs");
include!("ownership/lsp.rs");
include!("ownership/neovim.rs");

macro_rules! vscode_command_message_ids {
    () => {
        MsgId::VscodeCommandValidateTitle
            | MsgId::VscodeCommandCompileTitle
            | MsgId::VscodeCommandExtractTitle
            | MsgId::VscodeCommandWatchStartTitle
            | MsgId::VscodeCommandWatchStopTitle
            | MsgId::VscodeCommandRunTitle
            | MsgId::VscodeCommandTraceTitle
            | MsgId::VscodeCommandCliPathDescription
            | MsgId::VscodeCommandUntrusted
            | MsgId::VscodeCommandDocumentRequired
            | MsgId::VscodeCommandDocumentUnsaved
            | MsgId::VscodeCommandDocumentUntitled
            | MsgId::VscodeCommandDocumentChanged
            | MsgId::VscodeCommandDocumentOutsideRoot
            | MsgId::VscodeCommandWorkspaceRequired
            | MsgId::VscodeCommandCliPathInvalid
            | MsgId::VscodeCommandInputInvalid
            | MsgId::VscodeCommandWatchRunning
            | MsgId::VscodeCommandWatchNotRunning
            | MsgId::VscodeCommandWatchStopTimeout
            | MsgId::VscodeCommandResult
            | MsgId::VscodeCommandContentDiagnostics
            | MsgId::VscodeCommandFailure
            | MsgId::VscodeCommandProtocolFailure
            | MsgId::VscodeCommandWatchStatus
            | MsgId::VscodeCommandCompileOutputTitle
            | MsgId::VscodeCommandExtractOutputTitle
            | MsgId::VscodeCommandAssetTitle
            | MsgId::VscodeCommandAssetFilter
            | MsgId::VscodeCommandBlockTitle
            | MsgId::VscodeCommandBlockPrompt
            | MsgId::VscodeCommandBlockPlaceholder
            | MsgId::VscodeCommandFixtureTitle
            | MsgId::VscodeCommandFixtureFilter
            | MsgId::VscodeCommandRenameTitle
            | MsgId::VscodeCommandRenameCommandTitle
            | MsgId::VscodeCommandRenamePrompt
            | MsgId::VscodeCommandRenamePlaceholder
            | MsgId::VscodeCommandRenameBusy
            | MsgId::VscodeCommandRenameDocumentRequired
            | MsgId::VscodeCommandRenameUnavailable
            | MsgId::VscodeCommandRenameInvalid
            | MsgId::VscodeCommandRenameStale
            | MsgId::VscodeCommandRenameApplyFailed
            | MsgId::VscodeCommandRenameRequestFailed
    };
}

pub(super) const fn clients(id: MsgId) -> &'static [Client] {
    match id {
        tui_message_ids!() => &[Client::Tui],
        vscode_command_message_ids!() => &[Client::VsCode, Client::VsCodium],
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
