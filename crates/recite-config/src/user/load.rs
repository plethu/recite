use std::{
    fs, io,
    path::{Path, PathBuf},
};

use recite_ui::UiLocale;
use serde::Deserialize;

use super::model::UserConfigFieldPresence;
use super::{
    CONFIG_VERSION, ConfigError, ConfigFormat, ConfigProvenance, KeyHints, Keymap,
    LoadedUserConfig, PlayConfig, TuiColorMode, TuiContrast, UiConfig, UserConfig,
};
use crate::path::{
    ConfigPathSource, Platform, PlatformRoots, ResolvedConfigPath, resolve_config_path,
};

/// Loads user configuration through the production environment/platform adapter.
///
/// An absent platform default returns typed defaults. Any explicit override
/// failure is returned, including missing, unreadable, or malformed files.
pub fn load_user_config() -> Result<LoadedUserConfig, ConfigError> {
    super::UserConfigStore::discover()?.load()
}

/// Loads user configuration using synthetic platform inputs and a real local
/// file. It does not read ambient environment variables.
pub fn load_user_config_from(
    platform: Platform,
    roots: &PlatformRoots,
    explicit_override: Option<&Path>,
) -> Result<LoadedUserConfig, ConfigError> {
    let path = resolve_config_path(platform, roots, explicit_override)?;
    load_user_config_path(path.as_ref())
}

/// Loads a resolved path, preserving whether it was an explicit override or a
/// platform default. `None` means the platform supplied no configuration root.
pub fn load_user_config_path(
    path: Option<&ResolvedConfigPath>,
) -> Result<LoadedUserConfig, ConfigError> {
    load_source(path).map(|(loaded, _)| loaded)
}

pub(super) fn load_source(
    path: Option<&ResolvedConfigPath>,
) -> Result<(LoadedUserConfig, Option<String>), ConfigError> {
    let Some(path) = path else {
        return Ok((
            LoadedUserConfig::defaults(ConfigProvenance::Defaults, None),
            None,
        ));
    };

    let provenance = match path.source() {
        ConfigPathSource::ExplicitOverride => ConfigProvenance::ExplicitOverride,
        ConfigPathSource::PlatformDefault => ConfigProvenance::PlatformDefault,
    };
    let source = match fs::read_to_string(path.path()) {
        Ok(source) => source,
        Err(error) if error.kind() == io::ErrorKind::NotFound => {
            if path.is_explicit() {
                return Err(ConfigError::MissingExplicit {
                    path: path.path().to_path_buf(),
                });
            }
            return Ok((
                LoadedUserConfig::defaults(provenance, Some(path.path().to_path_buf())),
                None,
            ));
        }
        Err(error) => {
            return Err(ConfigError::Read {
                path: path.path().to_path_buf(),
                message: error.to_string(),
            });
        }
    };

    let loaded = parse_user_config(&source, path.path(), provenance)?;
    Ok((loaded, Some(source)))
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawUserConfig {
    config_version: Option<u32>,
    #[serde(default)]
    ui: RawUiConfig,
    #[serde(default)]
    play: RawPlayConfig,
    #[serde(default)]
    writer: RawWriterConfig,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawWriterConfig {
    confirm_exit: Option<bool>,
    view: Option<super::WriterView>,
    theme: Option<super::WriterTheme>,
    reduced_motion: Option<bool>,
    zoom_to_pointer: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawUiConfig {
    locale: Option<String>,
    keymap: Option<Keymap>,
    key_hints: Option<KeyHints>,
    color: Option<TuiColorMode>,
    contrast: Option<TuiContrast>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct RawPlayConfig {
    show_unavailable_choices: Option<bool>,
}

pub(super) fn parse_user_config(
    source: &str,
    path: &Path,
    provenance: ConfigProvenance,
) -> Result<LoadedUserConfig, ConfigError> {
    let raw = toml::from_str::<RawUserConfig>(source).map_err(|error| ConfigError::Malformed {
        path: path.to_path_buf(),
        message: error.to_string(),
    })?;

    let format = match raw.config_version {
        None => ConfigFormat::LegacyPreVersioned,
        Some(CONFIG_VERSION) => ConfigFormat::Versioned,
        Some(found) => {
            return Err(ConfigError::UnsupportedVersion {
                path: path.to_path_buf(),
                found,
                expected: CONFIG_VERSION,
            });
        }
    };

    let field_presence = UserConfigFieldPresence {
        writer_confirm_exit: raw.writer.confirm_exit.is_some(),
        writer_view: raw.writer.view.is_some(),
        writer_theme: raw.writer.theme.is_some(),
        writer_reduced_motion: raw.writer.reduced_motion.is_some(),
        writer_zoom_to_pointer: raw.writer.zoom_to_pointer.is_some(),

        ui_locale: raw.ui.locale.is_some(),
        keymap: raw.ui.keymap.is_some(),
        key_hints: raw.ui.key_hints.is_some(),
        color: raw.ui.color.is_some(),
        contrast: raw.ui.contrast.is_some(),
        show_unavailable_choices: raw.play.show_unavailable_choices.is_some(),
    };
    let defaults = UserConfig::default();
    let locale = match raw.ui.locale {
        Some(locale) => UiLocale::parse(&locale).map_err(|_| ConfigError::InvalidLocale {
            path: path.to_path_buf(),
            locale,
        })?,
        None => defaults.ui.locale,
    };
    let config = UserConfig {
        writer: super::WriterConfig {
            view: raw.writer.view.unwrap_or(defaults.writer.view),
            theme: raw.writer.theme.unwrap_or(defaults.writer.theme),
            reduced_motion: raw
                .writer
                .reduced_motion
                .unwrap_or(defaults.writer.reduced_motion),
            zoom_to_pointer: raw
                .writer
                .zoom_to_pointer
                .unwrap_or(defaults.writer.zoom_to_pointer),

            confirm_exit: raw
                .writer
                .confirm_exit
                .unwrap_or(defaults.writer.confirm_exit),
        },
        config_version: CONFIG_VERSION,
        ui: UiConfig {
            locale,
            keymap: raw.ui.keymap.unwrap_or(defaults.ui.keymap),
            key_hints: raw.ui.key_hints.unwrap_or(defaults.ui.key_hints),
            color: raw.ui.color.unwrap_or(defaults.ui.color),
            contrast: raw.ui.contrast.unwrap_or(defaults.ui.contrast),
        },
        play: PlayConfig {
            show_unavailable_choices: raw
                .play
                .show_unavailable_choices
                .unwrap_or(defaults.play.show_unavailable_choices),
        },
    };

    Ok(LoadedUserConfig {
        config,
        provenance,
        format,
        path: Some(PathBuf::from(path)),
        field_presence,
    })
}
