pub(super) const fn key(id: super::MsgId) -> Option<&'static str> {
    if let Some(key) = super::watch_message_ids::key(id) {
        return Some(key);
    }
    match id {
        super::MsgId::CliHelpAbout => Some("cli-help-about"),
        super::MsgId::CliHelpUsageHeading => Some("cli-help-usage-heading"),
        super::MsgId::CliHelpCommandsHeading => Some("cli-help-commands-heading"),
        super::MsgId::CliHelpArgumentsHeading => Some("cli-help-arguments-heading"),
        super::MsgId::CliHelpOptionsHeading => Some("cli-help-options-heading"),
        super::MsgId::CliHelpCommandValidate => Some("cli-help-command-validate"),
        super::MsgId::CliHelpCommandCompile => Some("cli-help-command-compile"),
        super::MsgId::CliHelpCommandExtract => Some("cli-help-command-extract"),
        super::MsgId::CliHelpCommandCheckIds => Some("cli-help-command-check-ids"),
        super::MsgId::CliHelpCommandCheckMarkup => Some("cli-help-command-check-markup"),
        super::MsgId::CliHelpCommandCheckMetadata => Some("cli-help-command-check-metadata"),
        super::MsgId::CliHelpCommandValidateProject => Some("cli-help-command-validate-project"),
        super::MsgId::CliHelpCommandCheckFresh => Some("cli-help-command-check-fresh"),
        super::MsgId::CliHelpCommandInspectSchema => Some("cli-help-command-inspect-schema"),
        super::MsgId::CliHelpCommandExplain => Some("cli-help-command-explain"),
        super::MsgId::CliHelpCommandWatch => Some("cli-help-command-watch"),
        super::MsgId::CliHelpCommandRun => Some("cli-help-command-run"),
        super::MsgId::CliHelpCommandTrace => Some("cli-help-command-trace"),
        super::MsgId::CliHelpCommandPlay => Some("cli-help-command-play"),
        super::MsgId::CliHelpCommandBench => Some("cli-help-command-bench"),
        super::MsgId::CliHelpArgPaths => Some("cli-help-arg-paths"),
        super::MsgId::CliHelpArgSchema => Some("cli-help-arg-schema"),
        super::MsgId::CliHelpArgSchemaInspection => Some("cli-help-arg-schema-inspection"),
        super::MsgId::CliHelpArgProjectRoot => Some("cli-help-arg-project-root"),
        super::MsgId::CliHelpArgDiagnosticCode => Some("cli-help-arg-diagnostic-code"),
        super::MsgId::CliHelpArgOutputCompile => Some("cli-help-arg-output-compile"),
        super::MsgId::CliHelpArgOutputExtract => Some("cli-help-arg-output-extract"),
        super::MsgId::CliHelpArgAssetRun => Some("cli-help-arg-asset-run"),
        super::MsgId::CliHelpArgAssetPlay => Some("cli-help-arg-asset-play"),
        super::MsgId::CliHelpArgBlock => Some("cli-help-arg-block"),
        super::MsgId::CliHelpArgFixture => Some("cli-help-arg-fixture"),
        super::MsgId::CliHelpArgUi => Some("cli-help-arg-ui"),
        super::MsgId::CliHelpArgKeymap => Some("cli-help-arg-keymap"),
        super::MsgId::CliHelpArgDialogueLocale => Some("cli-help-arg-dialogue-locale"),
        super::MsgId::CliHelpArgDialogueCatalog => Some("cli-help-arg-dialogue-catalog"),
        super::MsgId::CliHelpArgBenchScale => Some("cli-help-arg-bench-scale"),
        super::MsgId::CliHelpArgBenchGroup => Some("cli-help-arg-bench-group"),
        super::MsgId::CliHelpArgBenchFormat => Some("cli-help-arg-bench-format"),
        super::MsgId::CliHelpArgBenchOutput => Some("cli-help-arg-bench-output"),
        super::MsgId::CliHelpArgBenchBaseline => Some("cli-help-arg-bench-baseline"),
        super::MsgId::CliHelpArgBenchSamples => Some("cli-help-arg-bench-samples"),
        super::MsgId::ExplainCode => Some("explain-code"),
        super::MsgId::ExplainCategory => Some("explain-category"),
        super::MsgId::ExplainMeaning => Some("explain-meaning"),
        super::MsgId::ExplainCommonCauses => Some("explain-common-causes"),
        super::MsgId::ExplainHowToFix => Some("explain-how-to-fix"),
        super::MsgId::ExplainListItem => Some("explain-list-item"),
        super::MsgId::WatchEventError => Some("watch-event-error"),
        super::MsgId::CliHelpArgHelp => Some("cli-help-arg-help"),
        super::MsgId::CliHelpArgVersion => Some("cli-help-arg-version"),
        super::MsgId::PlayTuiStarting => Some("play-tui-starting"),
        super::MsgId::PlayStart => Some("play-start"),
        super::MsgId::PlayLine => Some("play-line"),
        super::MsgId::PlayPromptLine => Some("play-prompt-line"),
        super::MsgId::PlayPrompt => Some("play-prompt"),
        super::MsgId::PlayChoiceRow => Some("play-choice-row"),
        super::MsgId::PlayChoicePrompt => Some("play-choice-prompt"),
        super::MsgId::PlayConditionPrompt => Some("play-condition-prompt"),
        super::MsgId::PlayConditionResult => Some("play-condition-result"),
        super::MsgId::PlaySelectedChoice => Some("play-selected-choice"),
        super::MsgId::PlayEffect => Some("play-effect"),
        super::MsgId::PlayAckPrompt => Some("play-ack-prompt"),
        super::MsgId::PlayAckCompleted => Some("play-ack-completed"),
        super::MsgId::PlayEnd => Some("play-end"),
        super::MsgId::PlayDeferredEffects => Some("play-deferred-effects"),
        super::MsgId::PlayDeferredEffectRow => Some("play-deferred-effect-row"),
        super::MsgId::PlayInvalidInput => Some("play-invalid-input"),
        super::MsgId::PlayErrorEnterYOrN => Some("play-error-enter-y-or-n"),
        super::MsgId::PlayErrorEnterEnumVariant => Some("play-error-enter-enum-variant"),
        super::MsgId::PlayErrorPressEnterOrAck => Some("play-error-press-enter-or-ack"),
        super::MsgId::PlayErrorEmptyChoice => Some("play-error-empty-choice"),
        super::MsgId::PlayErrorChoiceIndexOutOfRange => {
            Some("play-error-choice-index-out-of-range")
        }
        super::MsgId::PlayErrorChoiceIdInvalid => Some("play-error-choice-id-invalid"),
        super::MsgId::PlayErrorChoiceIdUnavailable => Some("play-error-choice-id-unavailable"),
        super::MsgId::PlayErrorChoiceUnavailable => Some("play-error-choice-unavailable"),
        super::MsgId::PlayErrorChoiceUnavailableReason => {
            Some("play-error-choice-unavailable-reason")
        }
        super::MsgId::RunEffect => Some("run-effect"),
        super::MsgId::CliErrorPlayEof => Some("cli-error-play-eof"),
        super::MsgId::CliErrorPlayInvalidInput => Some("cli-error-play-invalid-input"),
        super::MsgId::CliErrorPlayInterrupted => Some("cli-error-play-interrupted"),
        super::MsgId::CliErrorPlayTuiRequiresTerminal => {
            Some("cli-error-play-tui-requires-terminal")
        }
        super::MsgId::CliErrorUiConfigRead => Some("cli-error-ui-config-read"),
        super::MsgId::CliErrorUiConfigToml => Some("cli-error-ui-config-toml"),
        super::MsgId::CliErrorUiLocaleInvalid => Some("cli-error-ui-locale-invalid"),
        super::MsgId::CliErrorDialogueCatalogConflict => {
            Some("cli-error-dialogue-catalog-conflict")
        }
        super::MsgId::CliErrorDialogueCatalogPluralFormsConflict => {
            Some("cli-error-dialogue-catalog-plural-forms-conflict")
        }
        super::MsgId::CliErrorDialogueCatalogMalformed => {
            Some("cli-error-dialogue-catalog-malformed")
        }
        super::MsgId::CliErrorDialogueCatalogMissingLocale => {
            Some("cli-error-dialogue-catalog-missing-locale")
        }
        super::MsgId::CliErrorDialogueCatalogSpecInvalid => {
            Some("cli-error-dialogue-catalog-spec-invalid")
        }
        super::MsgId::CliErrorDialogueLocaleInvalid => Some("cli-error-dialogue-locale-invalid"),
        super::MsgId::CliErrorGeneric => Some("cli-error-generic"),
        super::MsgId::CliErrorDiagnosticRendering => Some("cli-error-diagnostic-rendering"),
        super::MsgId::CliErrorDiagnosticCodeMalformed => {
            Some("cli-error-diagnostic-code-malformed")
        }
        super::MsgId::CliErrorDiagnosticCodeUnknown => Some("cli-error-diagnostic-code-unknown"),
        super::MsgId::CliErrorUiCatalog => Some("cli-error-ui-catalog"),
        super::MsgId::CliErrorBench => Some("cli-error-bench"),
        super::MsgId::CliErrorBenchmark => Some("cli-error-benchmark"),
        super::MsgId::CliErrorDialogueCatalogReasonExpectedDirective => {
            Some("cli-error-dialogue-catalog-reason-expected-directive")
        }
        super::MsgId::CliErrorDialogueCatalogReasonExpectedQuotedString => {
            Some("cli-error-dialogue-catalog-reason-expected-quoted-string")
        }
        super::MsgId::CliErrorDialogueCatalogReasonMissingContext => {
            Some("cli-error-dialogue-catalog-reason-missing-context")
        }
        super::MsgId::CliErrorDialogueCatalogReasonMissingId => {
            Some("cli-error-dialogue-catalog-reason-missing-id")
        }
        super::MsgId::CliErrorDialogueCatalogReasonMissingTranslation => {
            Some("cli-error-dialogue-catalog-reason-missing-translation")
        }
        super::MsgId::CliErrorDialogueCatalogReasonInvalidHeader => {
            Some("cli-error-dialogue-catalog-reason-invalid-header")
        }
        super::MsgId::CliErrorDialogueCatalogReasonInvalidPluralRule => {
            Some("cli-error-dialogue-catalog-reason-invalid-plural-rule")
        }
        super::MsgId::CliErrorDialogueCatalogReasonInvalidStableId => {
            Some("cli-error-dialogue-catalog-reason-invalid-stable-id")
        }
        super::MsgId::CliErrorDialogueCatalogReasonDuplicateField => {
            Some("cli-error-dialogue-catalog-reason-duplicate-field")
        }
        super::MsgId::CliErrorDialogueCatalogReasonDuplicateEntry => {
            Some("cli-error-dialogue-catalog-reason-duplicate-entry")
        }
        super::MsgId::CliErrorDialogueCatalogReasonInvalidFieldOrder => {
            Some("cli-error-dialogue-catalog-reason-invalid-field-order")
        }
        super::MsgId::CliErrorDialogueCatalogReasonPlaceholderMismatch => {
            Some("cli-error-dialogue-catalog-reason-placeholder-mismatch")
        }
        super::MsgId::CliErrorDialogueCatalogReasonPluralEntriesUnsupported => {
            Some("cli-error-dialogue-catalog-reason-plural-entries-unsupported")
        }
        super::MsgId::CliErrorDialogueCatalogReasonQuotedContinuationWithoutField => {
            Some("cli-error-dialogue-catalog-reason-quoted-continuation-without-field")
        }
        super::MsgId::CliErrorDialogueCatalogReasonUnexpectedTextAfterQuotedString => {
            Some("cli-error-dialogue-catalog-reason-unexpected-text-after-quoted-string")
        }
        super::MsgId::CliErrorDialogueCatalogReasonUnterminatedQuotedString => {
            Some("cli-error-dialogue-catalog-reason-unterminated-quoted-string")
        }
        super::MsgId::CliErrorDialogueCatalogReasonUnsupportedEscape => {
            Some("cli-error-dialogue-catalog-reason-unsupported-escape")
        }
        super::MsgId::CliErrorDecodeAsset => Some("cli-error-decode-asset"),
        super::MsgId::CliErrorAssetMetadata => Some("cli-error-asset-metadata"),
        super::MsgId::CliErrorAssetNotFile => Some("cli-error-asset-not-file"),
        super::MsgId::CliErrorMalformedCompiledAsset => Some("cli-error-malformed-compiled-asset"),
        super::MsgId::CliErrorDiagnostics => Some("cli-error-diagnostics"),
        super::MsgId::CliErrorFixtureChoiceIndex => Some("cli-error-fixture-choice-index"),
        super::MsgId::CliErrorFixtureChoiceNotInPrompt => {
            Some("cli-error-fixture-choice-not-in-prompt")
        }
        super::MsgId::CliErrorAmbiguousFixtureChoice => Some("cli-error-ambiguous-fixture-choice"),
        super::MsgId::CliErrorFixtureToml => Some("cli-error-fixture-toml"),
        super::MsgId::CliErrorMissingPath => Some("cli-error-missing-path"),
        super::MsgId::CliErrorMissingFixtureChoice => Some("cli-error-missing-fixture-choice"),
        super::MsgId::CliErrorNoInputs => Some("cli-error-no-inputs"),
        super::MsgId::CliErrorOutputOverwritesInput => Some("cli-error-output-overwrites-input"),
        super::MsgId::CliErrorBlockingEffect => Some("cli-error-blocking-effect"),
        super::MsgId::CliErrorBenchJson => Some("cli-error-bench-json"),
        super::MsgId::CliErrorTraceJson => Some("cli-error-trace-json"),
        super::MsgId::CliErrorSchemaInspectionJson => Some("cli-error-schema-inspection-json"),
        super::MsgId::CliErrorSchemaInspectionUnsupportedFormat => {
            Some("cli-error-schema-inspection-unsupported-format")
        }
        super::MsgId::CliErrorSchemaInspectionMalformed => {
            Some("cli-error-schema-inspection-malformed")
        }
        super::MsgId::CliErrorSchemaInspectionInvalidSummary => {
            Some("cli-error-schema-inspection-invalid-summary")
        }
        super::MsgId::CliErrorUnknownPrompt => Some("cli-error-unknown-prompt"),
        super::MsgId::CliErrorRead => Some("cli-error-read"),
        super::MsgId::CliErrorReadDir => Some("cli-error-read-dir"),
        super::MsgId::CliErrorWrite => Some("cli-error-write"),
        super::MsgId::CliErrorWatch => Some("cli-error-watch"),
        _ => None,
    }
}
