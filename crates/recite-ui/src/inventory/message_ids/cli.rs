pub(super) const fn key(id: super::MsgId) -> Option<&'static str> {
    if let Some(key) = super::watch_message_ids::key(id) {
        return Some(key);
    }
    match id {
        super::MsgId::CliHelpArgImportInput => Some("cli-help-arg-import-input"),
        super::MsgId::CliHelpArgImportFrom => Some("cli-help-arg-import-from"),
        super::MsgId::CliHelpArgImportMapping => Some("cli-help-arg-import-mapping"),
        super::MsgId::CliHelpArgImportOutputDir => Some("cli-help-arg-import-output-dir"),
        super::MsgId::CliHelpArgImportAcceptPartial => Some("cli-help-arg-import-accept-partial"),

        super::MsgId::CliImportSummary => Some("cli-import-summary"),
        super::MsgId::CliHelpCommandImport => Some("cli-help-command-import"),
        super::MsgId::CliHelpAbout => Some("cli-help-about"),
        super::MsgId::CliHelpUsageHeading => Some("cli-help-usage-heading"),
        super::MsgId::CliHelpCommandsHeading => Some("cli-help-commands-heading"),
        super::MsgId::CliHelpArgumentsHeading => Some("cli-help-arguments-heading"),
        super::MsgId::CliHelpOptionsHeading => Some("cli-help-options-heading"),
        super::MsgId::CliHelpCommandValidate => Some("cli-help-command-validate"),
        super::MsgId::CliHelpCommandCompile => Some("cli-help-command-compile"),
        super::MsgId::CliHelpCommandExtract => Some("cli-help-command-extract"),
        super::MsgId::CliHelpCommandId => Some("cli-help-command-id"),
        super::MsgId::CliHelpCommandProject => Some("cli-help-command-project"),
        super::MsgId::CliHelpCommandSchema => Some("cli-help-command-schema"),
        super::MsgId::CliHelpArgOnly => Some("cli-help-arg-only"),
        super::MsgId::CliHelpCommandIdCheck => Some("cli-help-command-id-check"),
        super::MsgId::CliHelpCommandIdAssign => Some("cli-help-command-id-assign"),
        super::MsgId::CliIdsAssigned => Some("cli-ids-assigned"),
        super::MsgId::CliIdsNoneMissing => Some("cli-ids-none-missing"),
        super::MsgId::CliIdsRemoved => Some("cli-ids-removed"),
        super::MsgId::CliHelpCommandIdRename => Some("cli-help-command-id-rename"),
        super::MsgId::CliHelpCommandIdReassign => Some("cli-help-command-id-reassign"),
        super::MsgId::CliHelpArgIdTarget => Some("cli-help-arg-id-target"),
        super::MsgId::CliHelpArgNewId => Some("cli-help-arg-new-id"),
        super::MsgId::CliHelpArgIdPaths => Some("cli-help-arg-id-paths"),
        super::MsgId::CliHelpArgCatalog => Some("cli-help-arg-catalog"),
        super::MsgId::CliIdsRenamed => Some("cli-ids-renamed"),
        super::MsgId::CliIdsReassignedDuplicate => Some("cli-ids-reassigned-duplicate"),
        super::MsgId::CliIdsNoCatalogs => Some("cli-ids-no-catalogs"),
        super::MsgId::CliHelpArgSince => Some("cli-help-arg-since"),
        super::MsgId::CliHelpArgAllowRemoved => Some("cli-help-arg-allow-removed"),
        super::MsgId::CliHelpCommandCatalog => Some("cli-help-command-catalog"),
        super::MsgId::CliHelpCommandCatalogStatus => Some("cli-help-command-catalog-status"),
        super::MsgId::CliHelpCommandCatalogRefresh => Some("cli-help-command-catalog-refresh"),
        super::MsgId::CliHelpArgCatalogInclude => Some("cli-help-arg-catalog-include"),
        super::MsgId::CliHelpArgCatalogCheck => Some("cli-help-arg-catalog-check"),
        super::MsgId::CliHelpArgCatalogFormat => Some("cli-help-arg-catalog-format"),
        super::MsgId::CliCatalogSummary => Some("cli-catalog-summary"),
        super::MsgId::CliCatalogNoLocale => Some("cli-catalog-no-locale"),
        super::MsgId::CliCatalogRefreshNeeded => Some("cli-catalog-refresh-needed"),
        super::MsgId::CliCatalogEntry => Some("cli-catalog-entry"),
        super::MsgId::CliCatalogStateMissing => Some("cli-catalog-state-missing"),
        super::MsgId::CliCatalogStateUntranslated => Some("cli-catalog-state-untranslated"),
        super::MsgId::CliCatalogStateNeedsReview => Some("cli-catalog-state-needs-review"),
        super::MsgId::CliCatalogStateSourceChanged => Some("cli-catalog-state-source-changed"),
        super::MsgId::CliCatalogRefreshed => Some("cli-catalog-refreshed"),
        super::MsgId::CliCatalogCurrent => Some("cli-catalog-current"),
        super::MsgId::CliHelpCommandRecording => Some("cli-help-command-recording"),
        super::MsgId::CliHelpCommandRecordingStatus => Some("cli-help-command-recording-status"),
        super::MsgId::CliHelpCommandRecordingExport => Some("cli-help-command-recording-export"),
        super::MsgId::CliHelpCommandRecordingImport => Some("cli-help-command-recording-import"),
        super::MsgId::CliHelpArgRecordingLocale => Some("cli-help-arg-recording-locale"),
        super::MsgId::CliHelpArgRecordingCheck => Some("cli-help-arg-recording-check"),
        super::MsgId::CliHelpArgRecordingBlock => Some("cli-help-arg-recording-block"),
        super::MsgId::CliHelpArgRecordingState => Some("cli-help-arg-recording-state"),
        super::MsgId::CliHelpArgRecordingHandoff => Some("cli-help-arg-recording-handoff"),
        super::MsgId::CliHelpArgRecordingFormat => Some("cli-help-arg-recording-format"),
        super::MsgId::CliHelpArgRecordingOutput => Some("cli-help-arg-recording-output"),
        super::MsgId::CliHelpArgRecordingFile => Some("cli-help-arg-recording-file"),
        super::MsgId::CliHelpArgRecordingApply => Some("cli-help-arg-recording-apply"),
        super::MsgId::CliRecordingSourceLanguage => Some("cli-recording-source-language"),
        super::MsgId::CliRecordingSummary => Some("cli-recording-summary"),
        super::MsgId::CliRecordingEntry => Some("cli-recording-entry"),
        super::MsgId::CliRecordingObsolete => Some("cli-recording-obsolete"),
        super::MsgId::CliRecordingStateToRecord => Some("cli-recording-state-to-record"),
        super::MsgId::CliRecordingStateStale => Some("cli-recording-state-stale"),
        super::MsgId::CliRecordingExportSelected => Some("cli-recording-export-selected"),
        super::MsgId::CliRecordingExportItem => Some("cli-recording-export-item"),
        super::MsgId::CliRecordingExportPreview => Some("cli-recording-export-preview"),
        super::MsgId::CliRecordingExportWritten => Some("cli-recording-export-written"),
        super::MsgId::CliRecordingImportRefused => Some("cli-recording-import-refused"),
        super::MsgId::CliRecordingReasonUnknownId => Some("cli-recording-reason-unknown-id"),
        super::MsgId::CliRecordingReasonWrongLocale => Some("cli-recording-reason-wrong-locale"),
        super::MsgId::CliRecordingReasonDuplicateTake => {
            Some("cli-recording-reason-duplicate-take")
        }
        super::MsgId::CliRecordingReasonNotReady => Some("cli-recording-reason-not-ready"),
        super::MsgId::CliRecordingReasonStale => Some("cli-recording-reason-stale"),
        super::MsgId::CliRecordingReasonMissingMedia => Some("cli-recording-reason-missing-media"),
        super::MsgId::CliRecordingImportSummary => Some("cli-recording-import-summary"),
        super::MsgId::CliRecordingImportPreview => Some("cli-recording-import-preview"),
        super::MsgId::CliRecordingImportApplied => Some("cli-recording-import-applied"),
        super::MsgId::CliHelpCommandProjectValidate => Some("cli-help-command-project-validate"),
        super::MsgId::CliHelpCommandProjectCheckFresh => {
            Some("cli-help-command-project-check-fresh")
        }
        super::MsgId::CliHelpCommandSchemaInspect => Some("cli-help-command-schema-inspect"),
        super::MsgId::CliHelpCommandSchemaExport => Some("cli-help-command-schema-export"),
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
        super::MsgId::CliHelpArgOutputSchema => Some("cli-help-arg-output-schema"),
        super::MsgId::CliHelpArgProducerKind => Some("cli-help-arg-producer-kind"),
        super::MsgId::CliHelpArgProducerId => Some("cli-help-arg-producer-id"),
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
        super::MsgId::CliHelpCommandSchemaCheckFresh => Some("cli-help-command-schema-check-fresh"),
        super::MsgId::CliHelpGroupAuthoring => Some("cli-help-group-authoring"),
        super::MsgId::CliHelpGroupBuilding => Some("cli-help-group-building"),
        super::MsgId::CliHelpGroupLocalisation => Some("cli-help-group-localisation"),
        super::MsgId::CliHelpGroupRunning => Some("cli-help-group-running"),
        super::MsgId::CliHelpGroupHelp => Some("cli-help-group-help"),
        super::MsgId::CliSuccessOk => Some("cli-success-ok"),
        super::MsgId::CliSuccessFiles => Some("cli-success-files"),
        super::MsgId::CliSuccessProject => Some("cli-success-project"),
        super::MsgId::CliDiagnosticErrors => Some("cli-diagnostic-errors"),
        super::MsgId::CliDiagnosticWarnings => Some("cli-diagnostic-warnings"),
        super::MsgId::CliDiagnosticNotes => Some("cli-diagnostic-notes"),
        super::MsgId::CliDiagnosticSummary => Some("cli-diagnostic-summary"),
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
        super::MsgId::CliErrorStableIdBaseline => Some("cli-error-stable-id-baseline"),
        super::MsgId::CliErrorStableIdReserved => Some("cli-error-stable-id-reserved"),
        super::MsgId::CliErrorStableIdMalformed => Some("cli-error-stable-id-malformed"),
        super::MsgId::CliErrorStableIdInUse => Some("cli-error-stable-id-in-use"),
        super::MsgId::CliErrorStableIdInCatalog => Some("cli-error-stable-id-in-catalog"),
        super::MsgId::CliErrorStableIdUnknown => Some("cli-error-stable-id-unknown"),
        super::MsgId::CliErrorStableIdAmbiguous => Some("cli-error-stable-id-ambiguous"),
        super::MsgId::CliErrorStableIdTarget => Some("cli-error-stable-id-target"),
        super::MsgId::CliErrorStableIdUnchanged => Some("cli-error-stable-id-unchanged"),
        super::MsgId::CliErrorStableIdCatalogMalformed => {
            Some("cli-error-stable-id-catalog-malformed")
        }
        super::MsgId::CliErrorCatalogMalformed => Some("cli-error-catalog-malformed"),
        super::MsgId::CliErrorCatalogRefresh => Some("cli-error-catalog-refresh"),
        super::MsgId::CliErrorCatalogChanged => Some("cli-error-catalog-changed"),
        super::MsgId::CliErrorRecordingLocale => Some("cli-error-recording-locale"),
        super::MsgId::CliErrorRecordingSidecar => Some("cli-error-recording-sidecar"),
        super::MsgId::CliErrorRecordingHandoff => Some("cli-error-recording-handoff"),
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
