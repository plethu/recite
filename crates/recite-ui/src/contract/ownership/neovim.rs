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
        | MsgId::NeovimCommandDescription
        | MsgId::NeovimCommandDocumentRequired
        | MsgId::NeovimCommandDocumentUnsaved
        | MsgId::NeovimCommandDocumentChanged
        | MsgId::NeovimCommandInputInvalid
        | MsgId::NeovimCommandCliMissing
        | MsgId::NeovimCommandOutputDerived
        | MsgId::NeovimCommandResult
        | MsgId::NeovimCommandContentDiagnostics
        | MsgId::NeovimCommandFailure
        | MsgId::NeovimCommandProtocolFailure
        | MsgId::NeovimCommandWatchRunning
        | MsgId::NeovimCommandWatchNotRunning
        | MsgId::NeovimCommandWatchStopTimeout
        | MsgId::NeovimCommandWatchStatus
    };
}
