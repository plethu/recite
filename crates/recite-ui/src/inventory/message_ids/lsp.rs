pub(super) const fn key(id: super::MsgId) -> Option<&'static str> {
    match id {
        super::MsgId::LspHoverRequires => Some("lsp-hover-requires"),
        super::MsgId::LspHoverIf => Some("lsp-hover-if"),
        super::MsgId::LspHoverSpeaker => Some("lsp-hover-speaker"),
        super::MsgId::LspHoverSpeakerWithDisplayName => Some("lsp-hover-speaker-with-display-name"),
        super::MsgId::LspHoverMetadata => Some("lsp-hover-metadata"),
        super::MsgId::LspHoverMetadataWithDomain => Some("lsp-hover-metadata-with-domain"),
        super::MsgId::LspHoverCondition => Some("lsp-hover-condition"),
        super::MsgId::LspHoverEffect => Some("lsp-hover-effect"),
        super::MsgId::LspHoverProjectionQuery => Some("lsp-hover-projection-query"),
        super::MsgId::LspHoverPresentationProjector => Some("lsp-hover-presentation-projector"),
        super::MsgId::LspHoverPresentationOutput => Some("lsp-hover-presentation-output"),
        super::MsgId::LspHoverPresentationLabel => Some("lsp-hover-presentation-label"),
        super::MsgId::LspHoverBlock => Some("lsp-hover-block"),
        super::MsgId::LspHoverRegistry => Some("lsp-hover-registry"),
        super::MsgId::LspHoverMetadataDomain => Some("lsp-hover-metadata-domain"),
        super::MsgId::LspHoverAvailabilityReason => Some("lsp-hover-availability-reason"),
        super::MsgId::LspHoverRegistryValue => Some("lsp-hover-registry-value"),
        super::MsgId::LspHoverEnumValue => Some("lsp-hover-enum-value"),
        super::MsgId::LspHoverDomainValue => Some("lsp-hover-domain-value"),
        super::MsgId::LspHoverProducedBy => Some("lsp-hover-produced-by"),
        super::MsgId::LspHoverSchemaProducer => Some("lsp-hover-schema-producer"),
        super::MsgId::LspHoverSchemaFreshness => Some("lsp-hover-schema-freshness"),
        super::MsgId::LspHoverSchemaFreshnessState => Some("lsp-hover-schema-freshness-state"),
        super::MsgId::LspHoverSchemaFreshnessStatus => Some("lsp-hover-schema-freshness-status"),
        super::MsgId::LspHoverSchemaFreshnessUnavailable => {
            Some("lsp-hover-schema-freshness-unavailable")
        }
        super::MsgId::LspHoverSchemaScopedFingerprints => {
            Some("lsp-hover-schema-scoped-fingerprints")
        }
        super::MsgId::LspCompletionAvailabilityReason => Some("lsp-completion-availability-reason"),
        super::MsgId::LspCompletionBlock => Some("lsp-completion-block"),
        super::MsgId::LspCompletionSpeaker => Some("lsp-completion-speaker"),
        super::MsgId::LspCompletionMetadataKey => Some("lsp-completion-metadata-key"),
        super::MsgId::LspCompletionMetadataKeyWithDomain => {
            Some("lsp-completion-metadata-key-with-domain")
        }
        super::MsgId::LspCompletionMetadataDomain => Some("lsp-completion-metadata-domain"),
        super::MsgId::LspCompletionCondition => Some("lsp-completion-condition"),
        super::MsgId::LspCompletionConditionDocumentation => {
            Some("lsp-completion-condition-documentation")
        }
        super::MsgId::LspCompletionEffect => Some("lsp-completion-effect"),
        super::MsgId::LspCompletionEffectDocumentation => {
            Some("lsp-completion-effect-documentation")
        }
        super::MsgId::LspCompletionProjectionQuery => Some("lsp-completion-projection-query"),
        super::MsgId::LspCompletionProjectionQueryDocumentation => {
            Some("lsp-completion-projection-query-documentation")
        }
        super::MsgId::LspCompletionProjectionQueryFunction => {
            Some("lsp-completion-projection-query-function")
        }
        super::MsgId::LspCompletionProjectionQueryCall => {
            Some("lsp-completion-projection-query-call")
        }
        super::MsgId::LspCompletionProjectionInput => Some("lsp-completion-projection-input"),
        super::MsgId::LspCompletionProjector => Some("lsp-completion-projector"),
        super::MsgId::LspCompletionOutput => Some("lsp-completion-output"),
        super::MsgId::LspCompletionLabel => Some("lsp-completion-label"),
        super::MsgId::LspCodeActionInsertMissingId => Some("lsp-code-action-insert-missing-id"),
        super::MsgId::LspCodeActionInsertAllMissingIds => {
            Some("lsp-code-action-insert-all-missing-ids")
        }
        super::MsgId::LspCodeActionCreateBlockStub => Some("lsp-code-action-create-block-stub"),
        super::MsgId::LspCodeActionAddCondition => Some("lsp-code-action-add-condition"),
        super::MsgId::LspCodeActionAddEffect => Some("lsp-code-action-add-effect"),
        super::MsgId::LspCodeActionSchemaAction => Some("lsp-code-action-schema-action"),
        super::MsgId::LspCodeActionSchemaDisabled => Some("lsp-code-action-schema-disabled"),
        super::MsgId::LspWarningUiConfig => Some("lsp-warning-ui-config"),
        _ => None,
    }
}
