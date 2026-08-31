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
