use std::{
    env, fs,
    path::{Path, PathBuf},
};

use serde::Deserialize;

use crate::{args::PlayKeymap, error::CliError, i18n::UiLocale};

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum Keymap {
    #[default]
    Standard,
    Vim,
}

impl From<PlayKeymap> for Keymap {
    fn from(keymap: PlayKeymap) -> Self {
        match keymap {
            PlayKeymap::Standard => Self::Standard,
            PlayKeymap::Vim => Self::Vim,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum KeyHints {
    #[default]
    Contextual,
    Compact,
    Hidden,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum TuiColorMode {
    #[default]
    Auto,
    Always,
    Never,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) enum TuiContrast {
    #[default]
    Standard,
    Accessible,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TuiSettings {
    pub(crate) locale: UiLocale,
    pub(crate) keymap: Keymap,
    pub(crate) key_hints: KeyHints,
    pub(crate) color: TuiColorMode,
    pub(crate) contrast: TuiContrast,
    pub(crate) show_unavailable_choices: bool,
}

impl Default for TuiSettings {
    fn default() -> Self {
        Self {
            locale: UiLocale::default(),
            keymap: Keymap::Standard,
            key_hints: KeyHints::Contextual,
            color: TuiColorMode::Auto,
            contrast: TuiContrast::Standard,
            show_unavailable_choices: true,
        }
    }
}

impl TuiSettings {
    pub(crate) fn load(keymap_override: Option<PlayKeymap>) -> Result<Self, CliError> {
        let mut settings = match config_path() {
            Some(path) if path.exists() => Self::load_path(&path)?,
            _ => Self::default(),
        };
        if let Some(keymap) = keymap_override {
            settings.keymap = keymap.into();
        }
        Ok(settings)
    }

    pub(super) fn load_path(path: &Path) -> Result<Self, CliError> {
        let source = fs::read_to_string(path).map_err(|source| CliError::TuiConfigRead {
            path: path.to_owned(),
            source,
        })?;
        let raw =
            toml::from_str::<RawConfig>(&source).map_err(|source| CliError::TuiConfigToml {
                path: path.to_owned(),
                source,
            })?;
        raw.into_settings(path)
    }

    pub(crate) fn color_enabled(&self) -> bool {
        self.color_enabled_with_env(|name| env::var_os(name))
    }

    pub(super) fn color_enabled_with_env(
        &self,
        env_var: impl Fn(&str) -> Option<std::ffi::OsString>,
    ) -> bool {
        match self.color {
            TuiColorMode::Always => true,
            TuiColorMode::Never => false,
            TuiColorMode::Auto => {
                env_var("NO_COLOR").is_none() && env_var("CLICOLOR") != Some("0".into())
            }
        }
    }

    pub(crate) fn help_locale() -> UiLocale {
        let Some(path) = config_path() else {
            return UiLocale::default();
        };
        let Ok(source) = fs::read_to_string(path) else {
            return UiLocale::default();
        };
        let Ok(raw) = toml::from_str::<RawConfig>(&source) else {
            return UiLocale::default();
        };
        raw.ui
            .locale
            .as_deref()
            .and_then(|locale| UiLocale::parse(locale).ok())
            .unwrap_or_default()
    }
}

fn config_path() -> Option<PathBuf> {
    if let Some(path) = env::var_os("RECITE_CONFIG") {
        return Some(PathBuf::from(path));
    }
    if let Some(config_home) = env::var_os("XDG_CONFIG_HOME") {
        return Some(
            PathBuf::from(config_home)
                .join("recite")
                .join("config.toml"),
        );
    }
    env::var_os("HOME").map(|home| {
        PathBuf::from(home)
            .join(".config")
            .join("recite")
            .join("config.toml")
    })
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawConfig {
    #[serde(default)]
    ui: RawUiConfig,
    #[serde(default)]
    play: RawPlayConfig,
}

impl RawConfig {
    fn into_settings(self, path: &Path) -> Result<TuiSettings, CliError> {
        let defaults = TuiSettings::default();
        let locale = match self.ui.locale {
            Some(locale) => UiLocale::parse(&locale).map_err(|()| CliError::UiLocaleInvalid {
                path: path.to_owned(),
                locale,
            })?,
            None => defaults.locale,
        };
        Ok(TuiSettings {
            locale,
            keymap: self.ui.keymap.unwrap_or(defaults.keymap),
            key_hints: self.ui.key_hints.unwrap_or(defaults.key_hints),
            color: self.ui.color.unwrap_or(defaults.color),
            contrast: self.ui.contrast.unwrap_or(defaults.contrast),
            show_unavailable_choices: self
                .play
                .show_unavailable_choices
                .unwrap_or(defaults.show_unavailable_choices),
        })
    }
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawUiConfig {
    locale: Option<String>,
    keymap: Option<Keymap>,
    key_hints: Option<KeyHints>,
    color: Option<TuiColorMode>,
    contrast: Option<TuiContrast>,
}

#[derive(Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPlayConfig {
    show_unavailable_choices: Option<bool>,
}

impl<'de> Deserialize<'de> for Keymap {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.as_str() {
            "standard" => Ok(Self::Standard),
            "vim" => Ok(Self::Vim),
            _ => Err(serde::de::Error::unknown_variant(
                &value,
                &["standard", "vim"],
            )),
        }
    }
}

impl<'de> Deserialize<'de> for KeyHints {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.as_str() {
            "contextual" => Ok(Self::Contextual),
            "compact" => Ok(Self::Compact),
            "hidden" => Ok(Self::Hidden),
            _ => Err(serde::de::Error::unknown_variant(
                &value,
                &["contextual", "compact", "hidden"],
            )),
        }
    }
}

impl<'de> Deserialize<'de> for TuiColorMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.as_str() {
            "auto" => Ok(Self::Auto),
            "always" => Ok(Self::Always),
            "never" => Ok(Self::Never),
            _ => Err(serde::de::Error::unknown_variant(
                &value,
                &["auto", "always", "never"],
            )),
        }
    }
}

impl<'de> Deserialize<'de> for TuiContrast {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = String::deserialize(deserializer)?;
        match value.as_str() {
            "standard" => Ok(Self::Standard),
            "accessible" => Ok(Self::Accessible),
            _ => Err(serde::de::Error::unknown_variant(
                &value,
                &["standard", "accessible"],
            )),
        }
    }
}
