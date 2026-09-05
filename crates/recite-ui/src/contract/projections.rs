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
    field: "editors/recite-neovim/lua/recite_messages.lua",
}];

const VSCODE_RUNTIME_PROJECTIONS: [ProjectionDeclaration; 2] = [
    ProjectionDeclaration {
        client: Client::VsCode,
        field: "editors/vscode/src/messages.generated.js",
    },
    ProjectionDeclaration {
        client: Client::VsCodium,
        field: "editors/vscode/src/messages.generated.js",
    },
];

const VSCODE_PACKAGE_PROJECTIONS: [ProjectionDeclaration; 2] = [
    ProjectionDeclaration {
        client: Client::VsCode,
        field: "editors/vscode/package.nls.json",
    },
    ProjectionDeclaration {
        client: Client::VsCodium,
        field: "editors/vscode/package.nls.json",
    },
];

const DISPLAY_NAME_PROJECTIONS: [ProjectionDeclaration; 5] = [
    ProjectionDeclaration {
        client: Client::Neovim,
        field: "editors/recite-neovim/lua/recite_messages.lua",
    },
    ProjectionDeclaration {
        client: Client::VsCode,
        field: "editors/vscode/src/messages.generated.js",
    },
    ProjectionDeclaration {
        client: Client::VsCodium,
        field: "editors/vscode/src/messages.generated.js",
    },
    ProjectionDeclaration {
        client: Client::VsCode,
        field: "editors/vscode/package.nls.json",
    },
    ProjectionDeclaration {
        client: Client::VsCodium,
        field: "editors/vscode/package.nls.json",
    },
];

const RESTART_EXHAUSTED_PROJECTIONS: [ProjectionDeclaration; 3] = [
    ProjectionDeclaration {
        client: Client::Neovim,
        field: "editors/recite-neovim/lua/recite_messages.lua",
    },
    ProjectionDeclaration {
        client: Client::VsCode,
        field: "editors/vscode/src/messages.generated.js",
    },
    ProjectionDeclaration {
        client: Client::VsCodium,
        field: "editors/vscode/src/messages.generated.js",
    },
];

/// Return the typed projection declarations for one canonical message ID.
///
/// The resource inventory and projection generator are checked against this
/// table by the contract test; neither may independently claim completeness.
pub(super) const fn for_message(id: MsgId) -> &'static [ProjectionDeclaration] {
    match id {
        // These shared lifecycle labels are consumed by both the LSP and its
        // Neovim host projection.
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
        | MsgId::NeovimHealthOpenBuffer => &NEOVIM_PROJECTION,
        MsgId::LspClientDisplayName => &DISPLAY_NAME_PROJECTIONS,
        MsgId::LspClientRestartExhausted => &RESTART_EXHAUSTED_PROJECTIONS,
        MsgId::LspClientStartFailed
        | MsgId::LspClientError
        | MsgId::LspClientExited
        | MsgId::LspClientRestartScheduled
        | MsgId::LspClientTransportFailed
        | MsgId::LspClientProtocolFailed
        | MsgId::LspClientLifecycleFailed
        | MsgId::LspClientActionStale
        | MsgId::LspClientActionClosed
        | MsgId::LspClientActionReopened
        | MsgId::LspClientActionExpired
        | MsgId::LspClientActionEvicted
        | MsgId::LspClientActionUnknown
        | MsgId::LspClientActionApplyFailed
        | MsgId::LspClientConfigPathInvalid
        | MsgId::LspClientConfigArgsInvalid
        | MsgId::LspClientConfigProjectRootInvalid
        | MsgId::LspClientConfigProjectRootNeedsWorkspace
        | MsgId::LspClientNotRunning => &VSCODE_RUNTIME_PROJECTIONS,
        MsgId::LspClientDescription
        | MsgId::LspClientUntrustedWorkspacesDescription
        | MsgId::LspClientConfigurationTitle
        | MsgId::LspClientConfigurationPathDescription
        | MsgId::LspClientConfigurationArgsDescription
        | MsgId::LspClientConfigurationProjectRootDescription => &VSCODE_PACKAGE_PROJECTIONS,
        MsgId::VscodeCommandValidateTitle
        | MsgId::VscodeCommandCompileTitle
        | MsgId::VscodeCommandExtractTitle
        | MsgId::VscodeCommandWatchStartTitle
        | MsgId::VscodeCommandWatchStopTitle
        | MsgId::VscodeCommandRunTitle
        | MsgId::VscodeCommandTraceTitle
        | MsgId::VscodeCommandRenameCommandTitle
        | MsgId::VscodeCommandCliPathDescription => &VSCODE_PACKAGE_PROJECTIONS,
        MsgId::VscodeCommandUntrusted
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
        | MsgId::VscodeCommandRenamePrompt
        | MsgId::VscodeCommandRenamePlaceholder
        | MsgId::VscodeCommandRenameBusy
        | MsgId::VscodeCommandRenameDocumentRequired
        | MsgId::VscodeCommandRenameUnavailable
        | MsgId::VscodeCommandRenameInvalid
        | MsgId::VscodeCommandRenameStale
        | MsgId::VscodeCommandRenameApplyFailed
        | MsgId::VscodeCommandRenameRequestFailed => &VSCODE_RUNTIME_PROJECTIONS,
        _ => &[],
    }
}
