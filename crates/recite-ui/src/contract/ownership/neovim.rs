macro_rules! neovim_message_ids {
    () => {
        MsgId::NeovimAutocmdDescription
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
        | MsgId::NeovimHealthOpenBuffer
    };
}
