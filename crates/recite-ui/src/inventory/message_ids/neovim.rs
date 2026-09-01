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
        _ => None,
    }
}
