pub(super) const fn key(id: super::MsgId) -> Option<&'static str> {
    match id {
        super::MsgId::VscodeCommandValidateTitle => Some("vscode-command-validate-title"),
        super::MsgId::VscodeCommandCompileTitle => Some("vscode-command-compile-title"),
        super::MsgId::VscodeCommandExtractTitle => Some("vscode-command-extract-title"),
        super::MsgId::VscodeCommandWatchStartTitle => Some("vscode-command-watch-start-title"),
        super::MsgId::VscodeCommandWatchStopTitle => Some("vscode-command-watch-stop-title"),
        super::MsgId::VscodeCommandRunTitle => Some("vscode-command-run-title"),
        super::MsgId::VscodeCommandTraceTitle => Some("vscode-command-trace-title"),
        super::MsgId::VscodeCommandCliPathDescription => {
            Some("vscode-command-cli-path-description")
        }
        super::MsgId::VscodeCommandUntrusted => Some("vscode-command-untrusted"),
        super::MsgId::VscodeCommandDocumentRequired => Some("vscode-command-document-required"),
        super::MsgId::VscodeCommandDocumentUnsaved => Some("vscode-command-document-unsaved"),
        super::MsgId::VscodeCommandDocumentUntitled => Some("vscode-command-document-untitled"),
        super::MsgId::VscodeCommandDocumentChanged => Some("vscode-command-document-changed"),
        super::MsgId::VscodeCommandDocumentOutsideRoot => {
            Some("vscode-command-document-outside-root")
        }
        super::MsgId::VscodeCommandWorkspaceRequired => Some("vscode-command-workspace-required"),
        super::MsgId::VscodeCommandCliPathInvalid => Some("vscode-command-cli-path-invalid"),
        super::MsgId::VscodeCommandInputInvalid => Some("vscode-command-input-invalid"),
        super::MsgId::VscodeCommandWatchRunning => Some("vscode-command-watch-running"),
        super::MsgId::VscodeCommandWatchNotRunning => Some("vscode-command-watch-not-running"),
        super::MsgId::VscodeCommandWatchStopTimeout => Some("vscode-command-watch-stop-timeout"),
        super::MsgId::VscodeCommandResult => Some("vscode-command-result"),
        super::MsgId::VscodeCommandContentDiagnostics => Some("vscode-command-content-diagnostics"),
        super::MsgId::VscodeCommandFailure => Some("vscode-command-failure"),
        super::MsgId::VscodeCommandProtocolFailure => Some("vscode-command-protocol-failure"),
        super::MsgId::VscodeCommandWatchStatus => Some("vscode-command-watch-status"),
        super::MsgId::VscodeCommandCompileOutputTitle => {
            Some("vscode-command-compile-output-title")
        }
        super::MsgId::VscodeCommandExtractOutputTitle => {
            Some("vscode-command-extract-output-title")
        }
        super::MsgId::VscodeCommandAssetTitle => Some("vscode-command-asset-title"),
        super::MsgId::VscodeCommandAssetFilter => Some("vscode-command-asset-filter"),
        super::MsgId::VscodeCommandBlockTitle => Some("vscode-command-block-title"),
        super::MsgId::VscodeCommandBlockPrompt => Some("vscode-command-block-prompt"),
        super::MsgId::VscodeCommandBlockPlaceholder => Some("vscode-command-block-placeholder"),
        super::MsgId::VscodeCommandFixtureTitle => Some("vscode-command-fixture-title"),
        super::MsgId::VscodeCommandFixtureFilter => Some("vscode-command-fixture-filter"),
        super::MsgId::VscodeCommandRenameTitle => Some("vscode-command-rename-title"),
        super::MsgId::VscodeCommandRenameCommandTitle => {
            Some("vscode-command-rename-command-title")
        }
        super::MsgId::VscodeCommandRenamePrompt => Some("vscode-command-rename-prompt"),
        super::MsgId::VscodeCommandRenamePlaceholder => Some("vscode-command-rename-placeholder"),
        super::MsgId::VscodeCommandRenameBusy => Some("vscode-command-rename-busy"),
        super::MsgId::VscodeCommandRenameDocumentRequired => {
            Some("vscode-command-rename-document-required")
        }
        super::MsgId::VscodeCommandRenameUnavailable => Some("vscode-command-rename-unavailable"),
        super::MsgId::VscodeCommandRenameInvalid => Some("vscode-command-rename-invalid"),
        super::MsgId::VscodeCommandRenameStale => Some("vscode-command-rename-stale"),
        super::MsgId::VscodeCommandRenameApplyFailed => Some("vscode-command-rename-apply-failed"),
        super::MsgId::VscodeCommandRenameRequestFailed => {
            Some("vscode-command-rename-request-failed")
        }
        _ => None,
    }
}
