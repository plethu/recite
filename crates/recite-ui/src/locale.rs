use std::{env, fmt};

use unic_langid::{LanguageIdentifier, langid};

pub const DEFAULT_LOCALE: &str = "en-US";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct UiLocaleError;

impl fmt::Display for UiLocaleError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str("invalid BCP-47 UI locale")
    }
}

impl std::error::Error for UiLocaleError {}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum UiLocale {
    Locale(LanguageIdentifier),
    System,
}

impl Default for UiLocale {
    fn default() -> Self {
        Self::Locale(langid!("en-US"))
    }
}

impl UiLocale {
    pub fn parse(value: &str) -> Result<Self, UiLocaleError> {
        if value == "system" {
            return Ok(Self::System);
        }
        value.parse().map(Self::Locale).map_err(|_| UiLocaleError)
    }

    pub fn resolve(&self) -> LanguageIdentifier {
        match self {
            Self::Locale(locale) => locale.clone(),
            Self::System => system_locale().unwrap_or_else(default_locale),
        }
    }
}

impl fmt::Display for UiLocale {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Locale(locale) => locale.fmt(formatter),
            Self::System => formatter.write_str("system"),
        }
    }
}

pub fn fallback_chain(requested: &LanguageIdentifier) -> Vec<LanguageIdentifier> {
    let mut locales = vec![requested.clone()];
    if requested.region.is_some() {
        let language_only = requested
            .language
            .to_string()
            .parse()
            .unwrap_or_else(|_| default_locale());
        if !locales.contains(&language_only) {
            locales.push(language_only);
        }
    }
    let default = default_locale();
    if !locales.contains(&default) {
        locales.push(default);
    }
    locales
}

fn default_locale() -> LanguageIdentifier {
    langid!("en-US")
}

fn system_locale() -> Option<LanguageIdentifier> {
    for key in ["LC_ALL", "LC_MESSAGES", "LANG"] {
        let Ok(value) = env::var(key) else { continue };
        let value = value.trim();
        if value.is_empty() || value == "C" || value == "POSIX" {
            continue;
        }
        let value = value
            .split('.')
            .next()
            .unwrap_or(value)
            .split('@')
            .next()
            .unwrap_or(value)
            .replace('_', "-");
        if let Ok(locale) = value.parse() {
            return Some(locale);
        }
    }
    None
}
