use std::env;

use recite_config::{
    InvocationOverrides, LoadedUserConfig, UiLocale, load_user_config, resolve_user_config,
};

use crate::{args::PlayKeymap, error::CliError};

pub(crate) use recite_config::{KeyHints, Keymap, TuiColorMode, TuiContrast};

impl From<PlayKeymap> for Keymap {
    fn from(keymap: PlayKeymap) -> Self {
        match keymap {
            PlayKeymap::Standard => Self::Standard,
            PlayKeymap::Vim => Self::Vim,
        }
    }
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
        let loaded = load_user_config()?;
        Ok(Self::from_loaded(&loaded, keymap_override))
    }

    pub(super) fn from_loaded(
        loaded: &LoadedUserConfig,
        keymap_override: Option<PlayKeymap>,
    ) -> Self {
        let invocation = keymap_override.map_or_else(InvocationOverrides::new, |keymap| {
            InvocationOverrides::new().with_keymap(keymap.into())
        });
        let resolved = resolve_user_config(loaded, &invocation);
        Self {
            locale: resolved.ui().locale().value().clone(),
            keymap: *resolved.ui().keymap().value(),
            key_hints: *resolved.ui().key_hints().value(),
            color: *resolved.ui().color().value(),
            contrast: *resolved.ui().contrast().value(),
            show_unavailable_choices: *resolved.show_unavailable_choices().value(),
        }
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
        Self::load(None)
            .map(|settings| settings.locale)
            .unwrap_or_default()
    }
}
