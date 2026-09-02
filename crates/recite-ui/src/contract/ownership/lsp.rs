macro_rules! lsp_message_ids {
    () => {
        MsgId::LspHoverRequires
        | MsgId::LspHoverIf
        | MsgId::LspHoverSpeaker
        | MsgId::LspHoverSpeakerWithDisplayName
        | MsgId::LspHoverMetadata
        | MsgId::LspHoverMetadataWithDomain
        | MsgId::LspHoverCondition
        | MsgId::LspHoverEffect
        | MsgId::LspHoverProjectionQuery
        | MsgId::LspHoverPresentationProjector
        | MsgId::LspHoverPresentationOutput
        | MsgId::LspHoverPresentationLabel
        | MsgId::LspHoverBlock
        | MsgId::LspHoverRegistry
        | MsgId::LspHoverMetadataDomain
        | MsgId::LspHoverAvailabilityReason
        | MsgId::LspHoverRegistryValue
        | MsgId::LspHoverEnumValue
        | MsgId::LspHoverDomainValue
        | MsgId::LspHoverProducedBy
        | MsgId::LspHoverSchemaProducer
        | MsgId::LspHoverSchemaFreshness
        | MsgId::LspHoverSchemaFreshnessState
        | MsgId::LspHoverSchemaFreshnessStatus
        | MsgId::LspHoverSchemaFreshnessUnavailable
        | MsgId::LspHoverSchemaScopedFingerprints
        | MsgId::LspCompletionAvailabilityReason
        | MsgId::LspCompletionBlock
        | MsgId::LspCompletionSpeaker
        | MsgId::LspCompletionMetadataKey
        | MsgId::LspCompletionMetadataKeyWithDomain
        | MsgId::LspCompletionMetadataDomain
        | MsgId::LspCompletionCondition
        | MsgId::LspCompletionConditionDocumentation
        | MsgId::LspCompletionEffect
        | MsgId::LspCompletionEffectDocumentation
        | MsgId::LspCompletionProjectionQuery
        | MsgId::LspCompletionProjectionQueryDocumentation
        | MsgId::LspCompletionProjectionQueryFunction
        | MsgId::LspCompletionProjectionQueryCall
        | MsgId::LspCompletionProjectionInput
        | MsgId::LspCompletionProjector
        | MsgId::LspCompletionOutput
        | MsgId::LspCompletionLabel
        | MsgId::LspCodeActionInsertMissingId
        | MsgId::LspCodeActionInsertAllMissingIds
        | MsgId::LspCodeActionCreateBlockStub
        | MsgId::LspCodeActionAddCondition
        | MsgId::LspCodeActionAddEffect
        | MsgId::LspCodeActionSchemaAction
        | MsgId::LspCodeActionSchemaDisabled
        | MsgId::LspWarningUiConfig
    };
}

macro_rules! lsp_client_message_ids {
    () => {
        MsgId::LspClientStartFailed
        | MsgId::LspClientError
        | MsgId::LspClientExited
        | MsgId::LspClientRestartScheduled
        | MsgId::LspClientTransportFailed
        | MsgId::LspClientProtocolFailed
        | MsgId::LspClientLifecycleFailed
        | MsgId::LspClientDescription
        | MsgId::LspClientUntrustedWorkspacesDescription
        | MsgId::LspClientConfigurationTitle
        | MsgId::LspClientConfigurationPathDescription
        | MsgId::LspClientConfigurationArgsDescription
        | MsgId::LspClientConfigurationProjectRootDescription
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
        | MsgId::LspClientNotRunning
    };
}
