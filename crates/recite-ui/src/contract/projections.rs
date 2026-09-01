use super::Client;
use crate::MsgId;

/// Typed metadata for one checked-in host projection of a Fluent message.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(super) struct ProjectionDeclaration {
    pub(super) client: Client,
    pub(super) field: &'static str,
}

const NEOVIM_PROJECTION: [ProjectionDeclaration; 1] = [ProjectionDeclaration {
    client: Client::Neovim,
    field: "editor/recite-neovim/lua/recite_messages.lua",
}];

/// Return the typed projection declarations for one canonical message ID.
///
/// The resource inventory and projection generator are checked against this
/// table by the contract test; neither may independently claim completeness.
pub(super) const fn for_message(id: MsgId) -> &'static [ProjectionDeclaration] {
    match id {
        // These shared lifecycle labels are consumed by both the LSP and its
        // Neovim host projection.
        MsgId::LspClientDisplayName
        | MsgId::LspClientRestartExhausted
        | MsgId::NeovimAutocmdDescription
        | MsgId::NeovimCallbackFailed
        | MsgId::NeovimHealthFiletypeOk
        | MsgId::NeovimHealthFiletypeError
        | MsgId::NeovimHealthLspExecutableFound
        | MsgId::NeovimHealthLspExecutableMissing
        | MsgId::NeovimHealthLspInstall
        | MsgId::NeovimHealthQueryFound
        | MsgId::NeovimHealthQueryMissing
        | MsgId::NeovimHealthParserFound
        | MsgId::NeovimHealthParserMissing
        | MsgId::NeovimHealthParserBuild
        | MsgId::NeovimHealthCurrentRoot
        | MsgId::NeovimHealthOpenBuffer => &NEOVIM_PROJECTION,
        _ => &[],
    }
}
