use std::{borrow::Cow, collections::BTreeMap, env, fmt};

use fluent_bundle::{FluentArgs, FluentBundle, FluentResource, FluentValue};
use unic_langid::{LanguageIdentifier, langid};

use crate::error::CliError;

pub(crate) const DEFAULT_LOCALE: &str = "en-US";

const DEFAULT_RESOURCE: &str = include_str!("../i18n/en-US.ftl");

struct EmbeddedCatalog {
    locale: &'static str,
    source: &'static str,
}

const EMBEDDED_CATALOGS: &[EmbeddedCatalog] = &[EmbeddedCatalog {
    locale: DEFAULT_LOCALE,
    source: DEFAULT_RESOURCE,
}];

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum UiLocale {
    Locale(LanguageIdentifier),
    System,
}

impl Default for UiLocale {
    fn default() -> Self {
        Self::Locale(default_langid())
    }
}

impl UiLocale {
    pub(crate) fn parse(value: &str) -> Result<Self, ()> {
        if value == "system" {
            return Ok(Self::System);
        }
        value
            .parse::<LanguageIdentifier>()
            .map(Self::Locale)
            .map_err(|_| ())
    }

    fn resolve(&self) -> LanguageIdentifier {
        match self {
            Self::Locale(locale) => locale.clone(),
            Self::System => system_locale().unwrap_or_else(default_langid),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MsgId {
    PlayTuiStarting,
    PlayStart,
    PlayLine,
    PlayPromptLine,
    PlayPrompt,
    PlayChoiceRow,
    PlayChoiceUnavailableSuffix,
    PlayChoicePrompt,
    PlayConditionPrompt,
    PlayConditionResult,
    PlaySelectedChoice,
    PlayEffect,
    PlayAckPrompt,
    PlayAckCompleted,
    PlayEnd,
    PlayDeferredEffects,
    PlayDeferredEffectRow,
    PlayInvalidInput,
    PlayErrorEnterYOrN,
    PlayErrorPressEnterOrAck,
    PlayErrorEmptyChoice,
    PlayErrorChoiceIndexOutOfRange,
    PlayErrorChoiceIdInvalid,
    PlayErrorChoiceIdUnavailable,
    PlayErrorChoiceUnavailable,
    PlayErrorChoiceUnavailableReason,
    TuiReady,
    TuiFinished,
    TuiCommand,
    TuiCommandWithValue,
    TuiUnknownCommand,
    TuiChoiceInputPrefix,
    TuiChoiceInput,
    TuiConditionYesRow,
    TuiConditionNoRow,
    TuiConditionYesShortcutRow,
    TuiConditionNoShortcutRow,
    TuiAckEnterHint,
    TuiHeaderTitle,
    TuiHeaderAsset,
    TuiHeaderBlock,
    TuiWaiting,
    TuiConditionTitle,
    TuiEffectTitle,
    TuiChoiceTitle,
    TuiMetadataMode,
    TuiMetadataRuntimeEffectId,
    TuiMetadataFunction,
    TuiMetadataArgs,
    TuiInputAnswer,
    TuiInputAck,
    TuiInputChoice,
    TuiChoiceUnavailable,
    TuiChoiceUnavailableReason,
    TuiTranscriptLine,
    TuiTranscriptPrompt,
    TuiTranscriptChoice,
    TuiTranscriptCondition,
    TuiTranscriptEffect,
    TuiTranscriptAck,
    TuiTranscriptDeferred,
    TuiTranscriptEnd,
    TuiTranscriptSelected,
    TuiTranscriptCompleted,
    TuiTranscriptDeferredEffects,
    TuiHelpTitle,
    TuiHelpKeyHeading,
    TuiHelpActionHeading,
    TuiHelpDescriptionHeading,
    TuiHelpActionClose,
    TuiHelpActionQuit,
    TuiHelpActionMove,
    TuiHelpActionSubmit,
    TuiHelpActionInput,
    TuiHelpActionShortcut,
    TuiHelpActionCommand,
    TuiHelpActionHelp,
    TuiHelpDescriptionClose,
    TuiHelpDescriptionOpenHelp,
    TuiHelpDescriptionQuit,
    TuiHelpDescriptionInterrupt,
    TuiHelpDescriptionMoveChoice,
    TuiHelpDescriptionSubmitChoice,
    TuiHelpDescriptionInputChoice,
    TuiHelpDescriptionMoveCondition,
    TuiHelpDescriptionShortcutCondition,
    TuiHelpDescriptionSubmitCondition,
    TuiHelpDescriptionSubmitEffect,
    TuiHelpDescriptionFinished,
    TuiHelpDescriptionCommand,
    TuiFooterCommand,
    CliErrorPlayEof,
    CliErrorPlayInvalidInput,
    CliErrorPlayInterrupted,
    CliErrorPlayTuiRequiresTerminal,
    CliErrorUiConfigRead,
    CliErrorUiConfigToml,
    CliErrorUiLocaleInvalid,
}

impl MsgId {
    pub(crate) const ALL: &'static [Self] = &[
        Self::PlayTuiStarting,
        Self::PlayStart,
        Self::PlayLine,
        Self::PlayPromptLine,
        Self::PlayPrompt,
        Self::PlayChoiceRow,
        Self::PlayChoiceUnavailableSuffix,
        Self::PlayChoicePrompt,
        Self::PlayConditionPrompt,
        Self::PlayConditionResult,
        Self::PlaySelectedChoice,
        Self::PlayEffect,
        Self::PlayAckPrompt,
        Self::PlayAckCompleted,
        Self::PlayEnd,
        Self::PlayDeferredEffects,
        Self::PlayDeferredEffectRow,
        Self::PlayInvalidInput,
        Self::PlayErrorEnterYOrN,
        Self::PlayErrorPressEnterOrAck,
        Self::PlayErrorEmptyChoice,
        Self::PlayErrorChoiceIndexOutOfRange,
        Self::PlayErrorChoiceIdInvalid,
        Self::PlayErrorChoiceIdUnavailable,
        Self::PlayErrorChoiceUnavailable,
        Self::PlayErrorChoiceUnavailableReason,
        Self::TuiReady,
        Self::TuiFinished,
        Self::TuiCommand,
        Self::TuiCommandWithValue,
        Self::TuiUnknownCommand,
        Self::TuiChoiceInputPrefix,
        Self::TuiChoiceInput,
        Self::TuiConditionYesRow,
        Self::TuiConditionNoRow,
        Self::TuiConditionYesShortcutRow,
        Self::TuiConditionNoShortcutRow,
        Self::TuiAckEnterHint,
        Self::TuiHeaderTitle,
        Self::TuiHeaderAsset,
        Self::TuiHeaderBlock,
        Self::TuiWaiting,
        Self::TuiConditionTitle,
        Self::TuiEffectTitle,
        Self::TuiChoiceTitle,
        Self::TuiMetadataMode,
        Self::TuiMetadataRuntimeEffectId,
        Self::TuiMetadataFunction,
        Self::TuiMetadataArgs,
        Self::TuiInputAnswer,
        Self::TuiInputAck,
        Self::TuiInputChoice,
        Self::TuiChoiceUnavailable,
        Self::TuiChoiceUnavailableReason,
        Self::TuiTranscriptLine,
        Self::TuiTranscriptPrompt,
        Self::TuiTranscriptChoice,
        Self::TuiTranscriptCondition,
        Self::TuiTranscriptEffect,
        Self::TuiTranscriptAck,
        Self::TuiTranscriptDeferred,
        Self::TuiTranscriptEnd,
        Self::TuiTranscriptSelected,
        Self::TuiTranscriptCompleted,
        Self::TuiTranscriptDeferredEffects,
        Self::TuiHelpTitle,
        Self::TuiHelpKeyHeading,
        Self::TuiHelpActionHeading,
        Self::TuiHelpDescriptionHeading,
        Self::TuiHelpActionClose,
        Self::TuiHelpActionQuit,
        Self::TuiHelpActionMove,
        Self::TuiHelpActionSubmit,
        Self::TuiHelpActionInput,
        Self::TuiHelpActionShortcut,
        Self::TuiHelpActionCommand,
        Self::TuiHelpActionHelp,
        Self::TuiHelpDescriptionClose,
        Self::TuiHelpDescriptionOpenHelp,
        Self::TuiHelpDescriptionQuit,
        Self::TuiHelpDescriptionInterrupt,
        Self::TuiHelpDescriptionMoveChoice,
        Self::TuiHelpDescriptionSubmitChoice,
        Self::TuiHelpDescriptionInputChoice,
        Self::TuiHelpDescriptionMoveCondition,
        Self::TuiHelpDescriptionShortcutCondition,
        Self::TuiHelpDescriptionSubmitCondition,
        Self::TuiHelpDescriptionSubmitEffect,
        Self::TuiHelpDescriptionFinished,
        Self::TuiHelpDescriptionCommand,
        Self::TuiFooterCommand,
        Self::CliErrorPlayEof,
        Self::CliErrorPlayInvalidInput,
        Self::CliErrorPlayInterrupted,
        Self::CliErrorPlayTuiRequiresTerminal,
        Self::CliErrorUiConfigRead,
        Self::CliErrorUiConfigToml,
        Self::CliErrorUiLocaleInvalid,
    ];

    pub(crate) fn key(self) -> &'static str {
        match self {
            Self::PlayTuiStarting => "play-tui-starting",
            Self::PlayStart => "play-start",
            Self::PlayLine => "play-line",
            Self::PlayPromptLine => "play-prompt-line",
            Self::PlayPrompt => "play-prompt",
            Self::PlayChoiceRow => "play-choice-row",
            Self::PlayChoiceUnavailableSuffix => "play-choice-unavailable-suffix",
            Self::PlayChoicePrompt => "play-choice-prompt",
            Self::PlayConditionPrompt => "play-condition-prompt",
            Self::PlayConditionResult => "play-condition-result",
            Self::PlaySelectedChoice => "play-selected-choice",
            Self::PlayEffect => "play-effect",
            Self::PlayAckPrompt => "play-ack-prompt",
            Self::PlayAckCompleted => "play-ack-completed",
            Self::PlayEnd => "play-end",
            Self::PlayDeferredEffects => "play-deferred-effects",
            Self::PlayDeferredEffectRow => "play-deferred-effect-row",
            Self::PlayInvalidInput => "play-invalid-input",
            Self::PlayErrorEnterYOrN => "play-error-enter-y-or-n",
            Self::PlayErrorPressEnterOrAck => "play-error-press-enter-or-ack",
            Self::PlayErrorEmptyChoice => "play-error-empty-choice",
            Self::PlayErrorChoiceIndexOutOfRange => "play-error-choice-index-out-of-range",
            Self::PlayErrorChoiceIdInvalid => "play-error-choice-id-invalid",
            Self::PlayErrorChoiceIdUnavailable => "play-error-choice-id-unavailable",
            Self::PlayErrorChoiceUnavailable => "play-error-choice-unavailable",
            Self::PlayErrorChoiceUnavailableReason => "play-error-choice-unavailable-reason",
            Self::TuiReady => "tui-ready",
            Self::TuiFinished => "tui-finished",
            Self::TuiCommand => "tui-command",
            Self::TuiCommandWithValue => "tui-command-with-value",
            Self::TuiUnknownCommand => "tui-unknown-command",
            Self::TuiChoiceInputPrefix => "tui-choice-input-prefix",
            Self::TuiChoiceInput => "tui-choice-input",
            Self::TuiConditionYesRow => "tui-condition-yes-row",
            Self::TuiConditionNoRow => "tui-condition-no-row",
            Self::TuiConditionYesShortcutRow => "tui-condition-yes-shortcut-row",
            Self::TuiConditionNoShortcutRow => "tui-condition-no-shortcut-row",
            Self::TuiAckEnterHint => "tui-ack-enter-hint",
            Self::TuiHeaderTitle => "tui-header-title",
            Self::TuiHeaderAsset => "tui-header-asset",
            Self::TuiHeaderBlock => "tui-header-block",
            Self::TuiWaiting => "tui-waiting",
            Self::TuiConditionTitle => "tui-condition-title",
            Self::TuiEffectTitle => "tui-effect-title",
            Self::TuiChoiceTitle => "tui-choice-title",
            Self::TuiMetadataMode => "tui-metadata-mode",
            Self::TuiMetadataRuntimeEffectId => "tui-metadata-runtime-effect-id",
            Self::TuiMetadataFunction => "tui-metadata-function",
            Self::TuiMetadataArgs => "tui-metadata-args",
            Self::TuiInputAnswer => "tui-input-answer",
            Self::TuiInputAck => "tui-input-ack",
            Self::TuiInputChoice => "tui-input-choice",
            Self::TuiChoiceUnavailable => "tui-choice-unavailable",
            Self::TuiChoiceUnavailableReason => "tui-choice-unavailable-reason",
            Self::TuiTranscriptLine => "tui-transcript-line",
            Self::TuiTranscriptPrompt => "tui-transcript-prompt",
            Self::TuiTranscriptChoice => "tui-transcript-choice",
            Self::TuiTranscriptCondition => "tui-transcript-condition",
            Self::TuiTranscriptEffect => "tui-transcript-effect",
            Self::TuiTranscriptAck => "tui-transcript-ack",
            Self::TuiTranscriptDeferred => "tui-transcript-deferred",
            Self::TuiTranscriptEnd => "tui-transcript-end",
            Self::TuiTranscriptSelected => "tui-transcript-selected",
            Self::TuiTranscriptCompleted => "tui-transcript-completed",
            Self::TuiTranscriptDeferredEffects => "tui-transcript-deferred-effects",
            Self::TuiHelpTitle => "tui-help-title",
            Self::TuiHelpKeyHeading => "tui-help-key-heading",
            Self::TuiHelpActionHeading => "tui-help-action-heading",
            Self::TuiHelpDescriptionHeading => "tui-help-description-heading",
            Self::TuiHelpActionClose => "tui-help-action-close",
            Self::TuiHelpActionQuit => "tui-help-action-quit",
            Self::TuiHelpActionMove => "tui-help-action-move",
            Self::TuiHelpActionSubmit => "tui-help-action-submit",
            Self::TuiHelpActionInput => "tui-help-action-input",
            Self::TuiHelpActionShortcut => "tui-help-action-shortcut",
            Self::TuiHelpActionCommand => "tui-help-action-command",
            Self::TuiHelpActionHelp => "tui-help-action-help",
            Self::TuiHelpDescriptionClose => "tui-help-description-close",
            Self::TuiHelpDescriptionOpenHelp => "tui-help-description-open-help",
            Self::TuiHelpDescriptionQuit => "tui-help-description-quit",
            Self::TuiHelpDescriptionInterrupt => "tui-help-description-interrupt",
            Self::TuiHelpDescriptionMoveChoice => "tui-help-description-move-choice",
            Self::TuiHelpDescriptionSubmitChoice => "tui-help-description-submit-choice",
            Self::TuiHelpDescriptionInputChoice => "tui-help-description-input-choice",
            Self::TuiHelpDescriptionMoveCondition => "tui-help-description-move-condition",
            Self::TuiHelpDescriptionShortcutCondition => "tui-help-description-shortcut-condition",
            Self::TuiHelpDescriptionSubmitCondition => "tui-help-description-submit-condition",
            Self::TuiHelpDescriptionSubmitEffect => "tui-help-description-submit-effect",
            Self::TuiHelpDescriptionFinished => "tui-help-description-finished",
            Self::TuiHelpDescriptionCommand => "tui-help-description-command",
            Self::TuiFooterCommand => "tui-footer-command",
            Self::CliErrorPlayEof => "cli-error-play-eof",
            Self::CliErrorPlayInvalidInput => "cli-error-play-invalid-input",
            Self::CliErrorPlayInterrupted => "cli-error-play-interrupted",
            Self::CliErrorPlayTuiRequiresTerminal => "cli-error-play-tui-requires-terminal",
            Self::CliErrorUiConfigRead => "cli-error-ui-config-read",
            Self::CliErrorUiConfigToml => "cli-error-ui-config-toml",
            Self::CliErrorUiLocaleInvalid => "cli-error-ui-locale-invalid",
        }
    }
}

pub(crate) struct Messages {
    requested: LanguageIdentifier,
    bundles: BTreeMap<String, FluentBundle<FluentResource>>,
}

impl Messages {
    pub(crate) fn load(locale: &UiLocale) -> Result<Self, CliError> {
        let resources = embedded_resources().map_err(|source| CliError::UiCatalog { source })?;
        Self::from_resources(locale.resolve(), resources)
            .map_err(|source| CliError::UiCatalog { source })
    }

    fn from_resources(
        requested: LanguageIdentifier,
        resources: impl IntoIterator<Item = (LanguageIdentifier, String)>,
    ) -> Result<Self, String> {
        let mut bundles = BTreeMap::new();
        for (locale, source) in resources {
            let locale_key = locale.to_string();
            let resource = match FluentResource::try_new(source) {
                Ok(resource) => resource,
                Err((_, errors)) if locale_key == DEFAULT_LOCALE => {
                    return Err(format!(
                        "failed to parse default Fluent resource: {errors:?}"
                    ));
                }
                Err(_) => continue,
            };
            let mut bundle = FluentBundle::new(vec![locale.clone()]);
            bundle.set_use_isolating(false);
            match bundle.add_resource(resource) {
                Ok(()) => {
                    bundles.insert(locale_key, bundle);
                }
                Err(errors) if locale_key == DEFAULT_LOCALE => {
                    return Err(format!("failed to add default Fluent resource: {errors:?}"));
                }
                Err(_) => continue,
            }
        }

        let default_key = DEFAULT_LOCALE.to_owned();
        let default = bundles
            .get(&default_key)
            .ok_or_else(|| format!("missing default Fluent catalog {DEFAULT_LOCALE}"))?;
        for id in MsgId::ALL {
            if default.get_message(id.key()).is_none() {
                return Err(format!("default Fluent catalog is missing {}", id.key()));
            }
        }

        Ok(Self { requested, bundles })
    }

    pub(crate) fn text(&self, id: MsgId) -> String {
        self.format(id, [])
    }

    pub(crate) fn format(
        &self,
        id: MsgId,
        args: impl IntoIterator<Item = (&'static str, String)>,
    ) -> String {
        let args = args.into_iter().collect::<Vec<_>>();
        for locale in fallback_chain(&self.requested) {
            let Some(bundle) = self.bundles.get(&locale.to_string()) else {
                continue;
            };
            let Some(message) = bundle.get_message(id.key()) else {
                continue;
            };
            let Some(pattern) = message.value() else {
                continue;
            };
            let mut fluent_args = FluentArgs::new();
            for (name, value) in &args {
                fluent_args.set(*name, FluentValue::String(Cow::Owned(value.clone())));
            }
            let mut errors = Vec::new();
            let formatted = bundle.format_pattern(pattern, Some(&fluent_args), &mut errors);
            if errors.is_empty() {
                return formatted.into_owned();
            }
        }
        id.key().to_owned()
    }
}

fn embedded_resources() -> Result<Vec<(LanguageIdentifier, String)>, String> {
    EMBEDDED_CATALOGS
        .iter()
        .map(|catalog| {
            catalog
                .locale
                .parse::<LanguageIdentifier>()
                .map(|locale| (locale, catalog.source.to_owned()))
                .map_err(|error| format!("invalid embedded locale {}: {error}", catalog.locale))
        })
        .collect()
}

pub(crate) fn default_langid() -> LanguageIdentifier {
    langid!("en-US")
}

fn fallback_chain(requested: &LanguageIdentifier) -> Vec<LanguageIdentifier> {
    let mut locales = vec![requested.clone()];
    if requested.region.is_some() {
        let language_only = requested
            .language
            .to_string()
            .parse()
            .unwrap_or_else(|_| default_langid());
        if !locales.contains(&language_only) {
            locales.push(language_only);
        }
    }
    let default = default_langid();
    if !locales.contains(&default) {
        locales.push(default);
    }
    locales
}

fn system_locale() -> Option<LanguageIdentifier> {
    for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        let Ok(value) = env::var(key) else {
            continue;
        };
        let value = value.trim();
        if value.is_empty() || value == "C" || value == "POSIX" {
            continue;
        }
        let locale = value
            .split('.')
            .next()
            .unwrap_or(value)
            .split('@')
            .next()
            .unwrap_or(value)
            .replace('_', "-");
        if let Ok(locale) = locale.parse::<LanguageIdentifier>() {
            return Some(locale);
        }
    }
    None
}

impl fmt::Display for UiLocale {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Locale(locale) => write!(formatter, "{locale}"),
            Self::System => formatter.write_str("system"),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn messages_with(
        requested: &str,
        resources: impl IntoIterator<Item = (&'static str, &'static str)>,
    ) -> Messages {
        Messages::from_resources(
            requested.parse().expect("requested locale"),
            resources
                .into_iter()
                .map(|(locale, source)| (locale.parse().expect("locale"), source.to_owned())),
        )
        .expect("messages load")
    }

    #[test]
    fn default_catalog_parses_and_contains_all_typed_messages() {
        let messages = Messages::load(&UiLocale::default()).expect("messages load");
        let default = messages
            .bundles
            .get(DEFAULT_LOCALE)
            .expect("default bundle exists");

        for id in MsgId::ALL {
            assert!(
                default.get_message(id.key()).is_some(),
                "missing {}",
                id.key()
            );
        }
    }

    #[test]
    fn formats_messages_with_variables() {
        let messages = Messages::load(&UiLocale::default()).expect("messages load");

        assert_eq!(
            messages.format(
                MsgId::PlayStart,
                [
                    ("asset", "asset-1".to_owned()),
                    ("block", "start".to_owned())
                ],
            ),
            "play asset=asset-1 block=start"
        );
    }

    #[test]
    fn missing_requested_message_falls_back_to_default_catalog() {
        let messages = messages_with(
            "en-GB",
            [
                ("en-US", DEFAULT_RESOURCE),
                ("en-GB", "other-message = Other\n"),
            ],
        );

        assert_eq!(
            messages.format(
                MsgId::PlayStart,
                [
                    ("asset", "asset-1".to_owned()),
                    ("block", "start".to_owned())
                ],
            ),
            "play asset=asset-1 block=start"
        );
    }

    #[test]
    fn malformed_non_default_catalog_falls_back_to_default_catalog() {
        let messages = messages_with(
            "en-GB",
            [
                ("en-US", DEFAULT_RESOURCE),
                ("en-GB", "not valid fluent = {"),
            ],
        );

        assert_eq!(
            messages.format(
                MsgId::PlayStart,
                [
                    ("asset", "asset-1".to_owned()),
                    ("block", "start".to_owned())
                ],
            ),
            "play asset=asset-1 block=start"
        );
    }

    #[test]
    fn parses_config_locale_values() {
        assert_eq!(
            UiLocale::parse("en-US").expect("locale").to_string(),
            "en-US"
        );
        assert_eq!(UiLocale::parse("system").expect("system"), UiLocale::System);
        assert!(UiLocale::parse("not a locale").is_err());
    }
}
