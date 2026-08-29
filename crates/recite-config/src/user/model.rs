use std::{fmt, path::PathBuf};

use recite_ui::UiLocale;
use serde::Deserialize;

mod presence;
pub(super) use presence::UserConfigFieldPresence;

/// The current user configuration format.
pub const CONFIG_VERSION: u32 = 1;

/// The four explicit authorities in the shared authoring architecture.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ConfigAuthority {
    /// Command-line and other per-invocation choices.
    Invocation,
    /// Project-owned dialogue and schema semantics.
    Project,
    /// Per-user presentation preferences represented by this crate.
    User,
    /// Derived, read-only compiler or tooling output.
    Generated,
}

/// User-owned fields understood by this crate.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum UserConfigField {
    /// Recite-owned UI locale, not dialogue locale.
    UiLocale,
    /// Interactive keymap preference.
    Keymap,
    /// Interactive key hint presentation preference.
    KeyHints,
    /// TUI colour policy.
    Color,
    /// TUI contrast policy.
    Contrast,
    /// Whether the play surface displays unavailable choices.
    ShowUnavailableChoices,
}

/// A typed, read-only user presentation configuration.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct UserConfig {
    /// Normalized current format version. Legacy input is reported separately
    /// in [`ConfigProvenance`] and is never written back implicitly.
    pub config_version: u32,
    /// Recite-owned presentation preferences.
    pub ui: UiConfig,
    /// Play-surface preferences.
    pub play: PlayConfig,
}

impl Default for UserConfig {
    fn default() -> Self {
        Self {
            config_version: CONFIG_VERSION,
            ui: UiConfig::default(),
            play: PlayConfig::default(),
        }
    }
}

/// UI preferences owned by the user rather than a project or dialogue asset.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct UiConfig {
    /// Explicit BCP-47 UI locale or the presentation-only `system` setting.
    pub locale: UiLocale,
    /// Interactive keymap.
    pub keymap: Keymap,
    /// How much key-help text to show.
    pub key_hints: KeyHints,
    /// TUI colour policy.
    pub color: TuiColorMode,
    /// TUI contrast policy.
    pub contrast: TuiContrast,
}

/// Play-surface preferences owned by the user.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct PlayConfig {
    /// Whether unavailable choices should remain visible in interactive play.
    pub show_unavailable_choices: bool,
}

impl Default for PlayConfig {
    fn default() -> Self {
        Self {
            show_unavailable_choices: true,
        }
    }
}

/// Interactive keymap choices.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum Keymap {
    /// Arrow-key and typed-entry controls.
    #[default]
    Standard,
    /// Vim-style normal, insert, and command modes.
    Vim,
}

/// Key-hint density choices.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum KeyHints {
    /// Context-sensitive hints.
    #[default]
    Contextual,
    /// Compact hints.
    Compact,
    /// No hints.
    Hidden,
}

/// TUI colour choices.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TuiColorMode {
    /// Follow `NO_COLOR` and `CLICOLOR` at the presentation edge.
    #[default]
    Auto,
    /// Always enable colour at the presentation edge.
    Always,
    /// Never enable colour.
    Never,
}

/// TUI contrast choices.
#[derive(Clone, Copy, Debug, Default, Eq, Ord, PartialEq, PartialOrd, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum TuiContrast {
    /// Standard palette.
    #[default]
    Standard,
    /// Higher-contrast palette.
    Accessible,
}

/// Where a loaded configuration came from.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ConfigProvenance {
    /// No usable default path existed, so in-memory defaults were used.
    Defaults,
    /// Values were supplied by a programmatic caller without a config file.
    Programmatic,
    /// The platform path was selected and loaded (or was absent).
    PlatformDefault,
    /// An explicit `$RECITE_CONFIG` path was selected and loaded.
    ExplicitOverride,
}

impl fmt::Display for ConfigProvenance {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Defaults => formatter.write_str("defaults"),
            Self::Programmatic => formatter.write_str("programmatic"),
            Self::PlatformDefault => formatter.write_str("platform default"),
            Self::ExplicitOverride => formatter.write_str("explicit override"),
        }
    }
}

/// Which input syntax was read.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum ConfigFormat {
    /// No file was read.
    Defaults,
    /// A pre-versioned file was read. It remains read-compatible only.
    LegacyPreVersioned,
    /// A file carrying [`CONFIG_VERSION`].
    Versioned,
}

/// A successfully loaded configuration and its source evidence.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct LoadedUserConfig {
    /// Typed user preferences.
    pub config: UserConfig,
    /// Source provenance.
    pub provenance: ConfigProvenance,
    /// Input syntax provenance.
    pub format: ConfigFormat,
    /// Selected path, if a platform or explicit path was available.
    pub path: Option<PathBuf>,
    pub(super) field_presence: UserConfigFieldPresence,
}

impl LoadedUserConfig {
    pub(super) fn defaults(provenance: ConfigProvenance, path: Option<PathBuf>) -> Self {
        Self {
            config: UserConfig::default(),
            provenance,
            format: ConfigFormat::Defaults,
            path,
            field_presence: UserConfigFieldPresence::default(),
        }
    }

    /// Creates a loaded representation for programmatically explicit values.
    /// Every field is marked explicit so a value equal to its default is not
    /// mistaken for an absent setting during resolution.
    #[must_use]
    pub fn from_explicit(config: UserConfig) -> Self {
        Self {
            config,
            provenance: ConfigProvenance::Programmatic,
            format: ConfigFormat::Defaults,
            path: None,
            field_presence: UserConfigFieldPresence::all_explicit(),
        }
    }

    /// Returns whether a user file or programmatic caller explicitly supplied
    /// this field.
    #[must_use]
    pub const fn field_is_explicit(&self, field: UserConfigField) -> bool {
        self.field_presence.is_explicit(field)
    }

    /// Whether the input was a legacy pre-versioned config file.
    #[must_use]
    pub const fn is_legacy(&self) -> bool {
        matches!(self.format, ConfigFormat::LegacyPreVersioned)
    }
}
