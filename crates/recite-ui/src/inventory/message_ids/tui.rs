pub(super) const fn key(id: super::MsgId) -> Option<&'static str> {
    match id {
        super::MsgId::TuiReady => Some("tui-ready"),
        super::MsgId::TuiFinished => Some("tui-finished"),
        super::MsgId::TuiCommand => Some("tui-command"),
        super::MsgId::TuiCommandWithValue => Some("tui-command-with-value"),
        super::MsgId::TuiUnknownCommand => Some("tui-unknown-command"),
        super::MsgId::TuiChoiceInputPrefix => Some("tui-choice-input-prefix"),
        super::MsgId::TuiChoiceInput => Some("tui-choice-input"),
        super::MsgId::TuiEnumVariantInput => Some("tui-enum-variant-input"),
        super::MsgId::TuiConditionYesRow => Some("tui-condition-yes-row"),
        super::MsgId::TuiConditionNoRow => Some("tui-condition-no-row"),
        super::MsgId::TuiConditionYesShortcutRow => Some("tui-condition-yes-shortcut-row"),
        super::MsgId::TuiConditionNoShortcutRow => Some("tui-condition-no-shortcut-row"),
        super::MsgId::TuiEnumConditionHint => Some("tui-enum-condition-hint"),
        super::MsgId::TuiAckEnterHint => Some("tui-ack-enter-hint"),
        super::MsgId::TuiHeaderTitle => Some("tui-header-title"),
        super::MsgId::TuiHeaderAsset => Some("tui-header-asset"),
        super::MsgId::TuiHeaderBlock => Some("tui-header-block"),
        super::MsgId::TuiWaiting => Some("tui-waiting"),
        super::MsgId::TuiMetadataMode => Some("tui-metadata-mode"),
        super::MsgId::TuiMetadataRuntimeEffectId => Some("tui-metadata-runtime-effect-id"),
        super::MsgId::TuiMetadataFunction => Some("tui-metadata-function"),
        super::MsgId::TuiMetadataArgs => Some("tui-metadata-args"),
        super::MsgId::TuiInputAnswer => Some("tui-input-answer"),
        super::MsgId::TuiInputEnumVariant => Some("tui-input-enum-variant"),
        super::MsgId::TuiInputAck => Some("tui-input-ack"),
        super::MsgId::TuiInputChoice => Some("tui-input-choice"),
        super::MsgId::TuiChoiceUnavailable => Some("tui-choice-unavailable"),
        super::MsgId::TuiChoiceUnavailableReason => Some("tui-choice-unavailable-reason"),
        super::MsgId::TuiDeferredQueueTitle => Some("tui-deferred-queue-title"),
        super::MsgId::TuiDeferredQueueScheduled => Some("tui-deferred-queue-scheduled"),
        super::MsgId::TuiDeferredQueueReadyAtEnd => Some("tui-deferred-queue-ready-at-end"),
        super::MsgId::TuiTranscriptLine => Some("tui-transcript-line"),
        super::MsgId::TuiTranscriptPrompt => Some("tui-transcript-prompt"),
        super::MsgId::TuiTranscriptChoice => Some("tui-transcript-choice"),
        super::MsgId::TuiTranscriptCondition => Some("tui-transcript-condition"),
        super::MsgId::TuiTranscriptEffect => Some("tui-transcript-effect"),
        super::MsgId::TuiTranscriptAck => Some("tui-transcript-ack"),
        super::MsgId::TuiTranscriptDeferred => Some("tui-transcript-deferred"),
        super::MsgId::TuiTranscriptEnd => Some("tui-transcript-end"),
        super::MsgId::TuiTranscriptCompleted => Some("tui-transcript-completed"),
        super::MsgId::TuiTranscriptEffectText => Some("tui-transcript-effect-text"),
        super::MsgId::TuiTranscriptDeferredEffectText => {
            Some("tui-transcript-deferred-effect-text")
        }
        super::MsgId::TuiTranscriptDeferredEffects => Some("tui-transcript-deferred-effects"),
        super::MsgId::TuiHelpTitle => Some("tui-help-title"),
        super::MsgId::TuiHelpKeyHeading => Some("tui-help-key-heading"),
        super::MsgId::TuiHelpActionHeading => Some("tui-help-action-heading"),
        super::MsgId::TuiHelpDescriptionHeading => Some("tui-help-description-heading"),
        super::MsgId::TuiHelpActionClose => Some("tui-help-action-close"),
        super::MsgId::TuiHelpActionQuit => Some("tui-help-action-quit"),
        super::MsgId::TuiHelpActionMove => Some("tui-help-action-move"),
        super::MsgId::TuiHelpActionSubmit => Some("tui-help-action-submit"),
        super::MsgId::TuiHelpActionInput => Some("tui-help-action-input"),
        super::MsgId::TuiHelpActionShortcut => Some("tui-help-action-shortcut"),
        super::MsgId::TuiHelpActionCommand => Some("tui-help-action-command"),
        super::MsgId::TuiHelpActionHelp => Some("tui-help-action-help"),
        super::MsgId::TuiHelpActionQueue => Some("tui-help-action-queue"),
        super::MsgId::TuiHelpDescriptionClose => Some("tui-help-description-close"),
        super::MsgId::TuiHelpDescriptionOpenHelp => Some("tui-help-description-open-help"),
        super::MsgId::TuiHelpDescriptionQuit => Some("tui-help-description-quit"),
        super::MsgId::TuiHelpDescriptionInterrupt => Some("tui-help-description-interrupt"),
        super::MsgId::TuiHelpDescriptionMoveChoice => Some("tui-help-description-move-choice"),
        super::MsgId::TuiHelpDescriptionSubmitChoice => Some("tui-help-description-submit-choice"),
        super::MsgId::TuiHelpDescriptionInputChoice => Some("tui-help-description-input-choice"),
        super::MsgId::TuiHelpDescriptionMoveCondition => {
            Some("tui-help-description-move-condition")
        }
        super::MsgId::TuiHelpDescriptionShortcutCondition => {
            Some("tui-help-description-shortcut-condition")
        }
        super::MsgId::TuiHelpDescriptionSubmitCondition => {
            Some("tui-help-description-submit-condition")
        }
        super::MsgId::TuiHelpDescriptionInputEnumCondition => {
            Some("tui-help-description-input-enum-condition")
        }
        super::MsgId::TuiHelpDescriptionSubmitEnumCondition => {
            Some("tui-help-description-submit-enum-condition")
        }
        super::MsgId::TuiHelpDescriptionSubmitEffect => Some("tui-help-description-submit-effect"),
        super::MsgId::TuiHelpDescriptionFinished => Some("tui-help-description-finished"),
        super::MsgId::TuiHelpDescriptionCommand => Some("tui-help-description-command"),
        super::MsgId::TuiHelpDescriptionQueue => Some("tui-help-description-queue"),
        super::MsgId::TuiFooterCommand => Some("tui-footer-command"),
        _ => None,
    }
}
