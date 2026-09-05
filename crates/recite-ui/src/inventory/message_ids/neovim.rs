pub(super) const fn key(id: super::MsgId) -> Option<&'static str> {
    match id {
        super::MsgId::NeovimAutocmdDescription => Some("neovim-autocmd-description"),
        super::MsgId::NeovimCallbackFailed => Some("neovim-callback-failed"),
        super::MsgId::NeovimHealthFiletypeOk => Some("neovim-health-filetype-ok"),
        super::MsgId::NeovimHealthFiletypeError => Some("neovim-health-filetype-error"),
        super::MsgId::NeovimHealthLspExecutableFound => Some("neovim-health-lsp-executable-found"),
        super::MsgId::NeovimHealthLspExecutableMissing => {
            Some("neovim-health-lsp-executable-missing")
        }
        super::MsgId::NeovimHealthLspInstall => Some("neovim-health-lsp-install"),
        super::MsgId::NeovimHealthQueryFound => Some("neovim-health-query-found"),
        super::MsgId::NeovimHealthQueryMissing => Some("neovim-health-query-missing"),
        super::MsgId::NeovimHealthParserFound => Some("neovim-health-parser-found"),
        super::MsgId::NeovimHealthParserMissing => Some("neovim-health-parser-missing"),
        super::MsgId::NeovimHealthParserBuild => Some("neovim-health-parser-build"),
        super::MsgId::NeovimHealthCurrentRoot => Some("neovim-health-current-root"),
        super::MsgId::NeovimHealthOpenBuffer => Some("neovim-health-open-buffer"),
        super::MsgId::NeovimCommandDescription => Some("neovim-command-description"),
        super::MsgId::NeovimCommandDocumentRequired => Some("neovim-command-document-required"),
        super::MsgId::NeovimCommandDocumentUnsaved => Some("neovim-command-document-unsaved"),
        super::MsgId::NeovimCommandDocumentChanged => Some("neovim-command-document-changed"),
        super::MsgId::NeovimCommandInputInvalid => Some("neovim-command-input-invalid"),
        super::MsgId::NeovimCommandCliMissing => Some("neovim-command-cli-missing"),
        super::MsgId::NeovimCommandOutputDerived => Some("neovim-command-output-derived"),
        super::MsgId::NeovimCommandResult => Some("neovim-command-result"),
        super::MsgId::NeovimCommandContentDiagnostics => Some("neovim-command-content-diagnostics"),
        super::MsgId::NeovimCommandFailure => Some("neovim-command-failure"),
        super::MsgId::NeovimCommandProtocolFailure => Some("neovim-command-protocol-failure"),
        super::MsgId::NeovimCommandWatchRunning => Some("neovim-command-watch-running"),
        super::MsgId::NeovimCommandWatchNotRunning => Some("neovim-command-watch-not-running"),
        super::MsgId::NeovimCommandWatchStopTimeout => Some("neovim-command-watch-stop-timeout"),
        super::MsgId::NeovimCommandWatchStatus => Some("neovim-command-watch-status"),
        _ => None,
    }
}
