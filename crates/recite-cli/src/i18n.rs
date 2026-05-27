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

macro_rules! message_ids {
    ($($variant:ident => $key:literal,)+) => {
        #[derive(Clone, Copy, Debug, Eq, PartialEq)]
        pub(crate) enum MsgId {
            $($variant,)+
        }

        impl MsgId {
            pub(crate) const ALL: &'static [Self] = &[
                $(Self::$variant,)+
            ];

            pub(crate) fn key(self) -> &'static str {
                match self {
                    $(Self::$variant => $key,)+
                }
            }
        }
    };
}

message_ids! {
    PlayTuiStarting => "play-tui-starting",
    PlayStart => "play-start",
    PlayLine => "play-line",
    PlayPromptLine => "play-prompt-line",
    PlayPrompt => "play-prompt",
    PlayChoiceRow => "play-choice-row",
    PlayChoiceUnavailableSuffix => "play-choice-unavailable-suffix",
    PlayChoicePrompt => "play-choice-prompt",
    PlayConditionPrompt => "play-condition-prompt",
    PlayConditionResult => "play-condition-result",
    PlaySelectedChoice => "play-selected-choice",
    PlayEffect => "play-effect",
    PlayAckPrompt => "play-ack-prompt",
    PlayAckCompleted => "play-ack-completed",
    PlayEnd => "play-end",
    PlayDeferredEffects => "play-deferred-effects",
    PlayDeferredEffectRow => "play-deferred-effect-row",
    PlayInvalidInput => "play-invalid-input",
    PlayErrorEnterYOrN => "play-error-enter-y-or-n",
    PlayErrorPressEnterOrAck => "play-error-press-enter-or-ack",
    PlayErrorEmptyChoice => "play-error-empty-choice",
    PlayErrorChoiceIndexOutOfRange => "play-error-choice-index-out-of-range",
    PlayErrorChoiceIdInvalid => "play-error-choice-id-invalid",
    PlayErrorChoiceIdUnavailable => "play-error-choice-id-unavailable",
    PlayErrorChoiceUnavailable => "play-error-choice-unavailable",
    PlayErrorChoiceUnavailableReason => "play-error-choice-unavailable-reason",
    TuiReady => "tui-ready",
    TuiFinished => "tui-finished",
    TuiCommand => "tui-command",
    TuiCommandWithValue => "tui-command-with-value",
    TuiUnknownCommand => "tui-unknown-command",
    TuiChoiceInputPrefix => "tui-choice-input-prefix",
    TuiChoiceInput => "tui-choice-input",
    TuiConditionYesRow => "tui-condition-yes-row",
    TuiConditionNoRow => "tui-condition-no-row",
    TuiConditionYesShortcutRow => "tui-condition-yes-shortcut-row",
    TuiConditionNoShortcutRow => "tui-condition-no-shortcut-row",
    TuiAckEnterHint => "tui-ack-enter-hint",
    TuiHeaderTitle => "tui-header-title",
    TuiHeaderAsset => "tui-header-asset",
    TuiHeaderBlock => "tui-header-block",
    TuiWaiting => "tui-waiting",
    TuiMetadataMode => "tui-metadata-mode",
    TuiMetadataRuntimeEffectId => "tui-metadata-runtime-effect-id",
    TuiMetadataFunction => "tui-metadata-function",
    TuiMetadataArgs => "tui-metadata-args",
    TuiInputAnswer => "tui-input-answer",
    TuiInputAck => "tui-input-ack",
    TuiInputChoice => "tui-input-choice",
    TuiChoiceUnavailable => "tui-choice-unavailable",
    TuiChoiceUnavailableReason => "tui-choice-unavailable-reason",
    TuiDeferredQueueTitle => "tui-deferred-queue-title",
    TuiDeferredQueueScheduled => "tui-deferred-queue-scheduled",
    TuiDeferredQueueReadyAtEnd => "tui-deferred-queue-ready-at-end",
    TuiTranscriptLine => "tui-transcript-line",
    TuiTranscriptPrompt => "tui-transcript-prompt",
    TuiTranscriptChoice => "tui-transcript-choice",
    TuiTranscriptCondition => "tui-transcript-condition",
    TuiTranscriptEffect => "tui-transcript-effect",
    TuiTranscriptAck => "tui-transcript-ack",
    TuiTranscriptDeferred => "tui-transcript-deferred",
    TuiTranscriptEnd => "tui-transcript-end",
    TuiTranscriptCompleted => "tui-transcript-completed",
    TuiTranscriptDeferredEffects => "tui-transcript-deferred-effects",
    TuiHelpTitle => "tui-help-title",
    TuiHelpKeyHeading => "tui-help-key-heading",
    TuiHelpActionHeading => "tui-help-action-heading",
    TuiHelpDescriptionHeading => "tui-help-description-heading",
    TuiHelpActionClose => "tui-help-action-close",
    TuiHelpActionQuit => "tui-help-action-quit",
    TuiHelpActionMove => "tui-help-action-move",
    TuiHelpActionSubmit => "tui-help-action-submit",
    TuiHelpActionInput => "tui-help-action-input",
    TuiHelpActionShortcut => "tui-help-action-shortcut",
    TuiHelpActionCommand => "tui-help-action-command",
    TuiHelpActionHelp => "tui-help-action-help",
    TuiHelpActionQueue => "tui-help-action-queue",
    TuiHelpDescriptionClose => "tui-help-description-close",
    TuiHelpDescriptionOpenHelp => "tui-help-description-open-help",
    TuiHelpDescriptionQuit => "tui-help-description-quit",
    TuiHelpDescriptionInterrupt => "tui-help-description-interrupt",
    TuiHelpDescriptionMoveChoice => "tui-help-description-move-choice",
    TuiHelpDescriptionSubmitChoice => "tui-help-description-submit-choice",
    TuiHelpDescriptionInputChoice => "tui-help-description-input-choice",
    TuiHelpDescriptionMoveCondition => "tui-help-description-move-condition",
    TuiHelpDescriptionShortcutCondition => "tui-help-description-shortcut-condition",
    TuiHelpDescriptionSubmitCondition => "tui-help-description-submit-condition",
    TuiHelpDescriptionSubmitEffect => "tui-help-description-submit-effect",
    TuiHelpDescriptionFinished => "tui-help-description-finished",
    TuiHelpDescriptionCommand => "tui-help-description-command",
    TuiHelpDescriptionQueue => "tui-help-description-queue",
    TuiFooterCommand => "tui-footer-command",
    CliErrorPlayEof => "cli-error-play-eof",
    CliErrorPlayInvalidInput => "cli-error-play-invalid-input",
    CliErrorPlayInterrupted => "cli-error-play-interrupted",
    CliErrorPlayTuiRequiresTerminal => "cli-error-play-tui-requires-terminal",
    CliErrorUiConfigRead => "cli-error-ui-config-read",
    CliErrorUiConfigToml => "cli-error-ui-config-toml",
    CliErrorUiLocaleInvalid => "cli-error-ui-locale-invalid",
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
