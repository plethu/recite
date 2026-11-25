macro_rules! save_message_ids {
    () => {
        MsgId::CliHelpCommandSave
        | MsgId::CliHelpCommandSaveImpact
        | MsgId::CliHelpCommandSaveInspect
        | MsgId::CliHelpArgSaveCandidate
        | MsgId::CliHelpArgSaveBaseline
        | MsgId::CliHelpArgSaveSince
        | MsgId::CliHelpArgSaveCheck
        | MsgId::CliHelpArgAllowFallback
        | MsgId::CliHelpArgSaveFormat
        | MsgId::CliHelpArgSaveFile
        | MsgId::CliHelpArgSaveAsset
        | MsgId::CliSaveImpactIdentical
        | MsgId::CliSaveImpactSummary
        | MsgId::CliSaveImpactPrompts
        | MsgId::CliSaveImpactFallback
        | MsgId::CliSaveImpactAcknowledged
        | MsgId::CliSaveImpactUnacknowledged
        | MsgId::CliSavePointBlockStart
        | MsgId::CliSavePointAfterLine
        | MsgId::CliSavePointPrompt
        | MsgId::CliSavePointBlockingEffect
        | MsgId::CliSavePointAfterEffect
        | MsgId::CliSaveReasonBlockRemoved
        | MsgId::CliSaveReasonLandmarkRemoved
        | MsgId::CliSaveReasonLandmarkMoved
        | MsgId::CliSaveReasonLandmarkAmbiguous
        | MsgId::CliSaveReasonStatementsChanged
        | MsgId::CliSaveReasonBranchRemoved
        | MsgId::CliSaveReasonLevelRefused
        | MsgId::CliSaveInspectSummary
        | MsgId::CliSaveInspectPositionStart
        | MsgId::CliSaveInspectPositionAfter
        | MsgId::CliSaveInspectPositionEnded
        | MsgId::CliSaveInspectPendingPrompt
        | MsgId::CliSaveInspectPendingEffect
        | MsgId::CliSaveInspectDeferred
        | MsgId::CliSaveInspectEnded
        | MsgId::CliSaveInspectResumes
        | MsgId::CliSaveInspectReevaluated
        | MsgId::CliSaveInspectFallback
        | MsgId::CliSaveBaselineDiagnostics
        | MsgId::CliSaveInspectRestoresNothing
        | MsgId::CliErrorSaveBaseline
        | MsgId::CliErrorSaveSinceNeedsProject
        | MsgId::CliErrorSaveMissingAsset
        | MsgId::CliErrorSaveBuild
        | MsgId::CliErrorSaveSnapshot
        | MsgId::CliHelpArgWatchSaveBaseline
        | MsgId::CliSaveReasonStatementsChangedStart
    };
}
