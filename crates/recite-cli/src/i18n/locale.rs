use std::{env, fmt};

use unic_langid::{LanguageIdentifier, langid};

pub(crate) const DEFAULT_LOCALE: &str = "en-US";

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

    pub(super) fn resolve(&self) -> LanguageIdentifier {
        match self {
            Self::Locale(locale) => locale.clone(),
            Self::System => system_locale().unwrap_or_else(default_langid),
        }
    }
}

impl fmt::Display for UiLocale {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Locale(locale) => write!(formatter, "{locale}"),
            Self::System => formatter.write_str("system"),
        }
    }
}

pub(super) fn default_langid() -> LanguageIdentifier {
    langid!("en-US")
}

pub(super) fn fallback_chain(requested: &LanguageIdentifier) -> Vec<LanguageIdentifier> {
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
