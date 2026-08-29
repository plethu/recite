use super::super::model::{
    ConfigAuthority, KeyHints, Keymap, PlayConfig, TuiColorMode, TuiContrast, UserConfig,
};
use super::policy::{
    AuthorityValue, ColorPolicy, ContrastPolicy, KeyHintsPolicy, KeymapPolicy, ResolvedField,
    ShowUnavailableChoicesPolicy, UiLocalePolicy, resolve_field,
};
use recite_ui::UiLocale;

/// Invocation-owned overrides. The type exposes only the already-settled
/// invocation override, keymap; presentation fields remain user-only.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct InvocationOverrides {
    keymap: Option<Keymap>,
}

impl InvocationOverrides {
    /// Creates an invocation with no overrides.
    #[must_use]
    pub const fn new() -> Self {
        Self { keymap: None }
    }

    /// Sets the invocation-owned keymap override.
    #[must_use]
    pub const fn with_keymap(mut self, keymap: Keymap) -> Self {
        self.keymap = Some(keymap);
        self
    }

    /// Returns the optional invocation keymap override.
    #[must_use]
    pub const fn keymap(&self) -> Option<Keymap> {
        self.keymap
    }
}

/// Fully resolved user presentation settings with per-field provenance.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ResolvedUserConfig {
    ui_locale: ResolvedField<UiLocale>,
    keymap: ResolvedField<Keymap>,
    key_hints: ResolvedField<KeyHints>,
    color: ResolvedField<TuiColorMode>,
    contrast: ResolvedField<TuiContrast>,
    show_unavailable_choices: ResolvedField<bool>,
}

impl ResolvedUserConfig {
    /// Returns the resolved UI settings.
    #[must_use]
    pub const fn ui(&self) -> ResolvedUiConfig<'_> {
        ResolvedUiConfig { config: self }
    }

    /// Returns the resolved play setting.
    #[must_use]
    pub const fn show_unavailable_choices(&self) -> &ResolvedField<bool> {
        &self.show_unavailable_choices
    }
}

/// Borrowed resolved UI settings.
#[derive(Clone, Copy, Debug)]
pub struct ResolvedUiConfig<'a> {
    config: &'a ResolvedUserConfig,
}

impl ResolvedUiConfig<'_> {
    /// Returns the resolved locale.
    #[must_use]
    pub const fn locale(&self) -> &ResolvedField<UiLocale> {
        &self.config.ui_locale
    }

    /// Returns the resolved keymap.
    #[must_use]
    pub const fn keymap(&self) -> &ResolvedField<Keymap> {
        &self.config.keymap
    }

    /// Returns the resolved key-hint preference.
    #[must_use]
    pub const fn key_hints(&self) -> &ResolvedField<KeyHints> {
        &self.config.key_hints
    }

    /// Returns the resolved colour preference.
    #[must_use]
    pub const fn color(&self) -> &ResolvedField<TuiColorMode> {
        &self.config.color
    }

    /// Returns the resolved contrast preference.
    #[must_use]
    pub const fn contrast(&self) -> &ResolvedField<TuiContrast> {
        &self.config.contrast
    }
}

/// Apply the named user-field policies. No project or generated values enter
/// this resolution graph, keeping dialogue semantics outside user config.
pub fn resolve_user_config(
    config: &UserConfig,
    invocation: &InvocationOverrides,
) -> ResolvedUserConfig {
    let ui = &config.ui;
    let play = &config.play;
    ResolvedUserConfig {
        ui_locale: resolve_field(
            UiLocalePolicy,
            UiLocale::default(),
            [AuthorityValue::new(
                ConfigAuthority::User,
                ui.locale.clone(),
            )],
        )
        .unwrap_or_else(|_| unreachable!("the named user policy permits user values")),
        keymap: resolve_field(
            KeymapPolicy,
            Keymap::default(),
            std::iter::once(AuthorityValue::new(ConfigAuthority::User, ui.keymap)).chain(
                invocation
                    .keymap
                    .into_iter()
                    .map(|value| AuthorityValue::new(ConfigAuthority::Invocation, value)),
            ),
        )
        .unwrap_or_else(|_| {
            unreachable!("the named keymap policy permits invocation and user values")
        }),
        key_hints: resolve_field(
            KeyHintsPolicy,
            KeyHints::default(),
            [AuthorityValue::new(ConfigAuthority::User, ui.key_hints)],
        )
        .unwrap_or_else(|_| unreachable!("the named user policy permits user values")),
        color: resolve_field(
            ColorPolicy,
            TuiColorMode::default(),
            [AuthorityValue::new(ConfigAuthority::User, ui.color)],
        )
        .unwrap_or_else(|_| unreachable!("the named user policy permits user values")),
        contrast: resolve_field(
            ContrastPolicy,
            TuiContrast::default(),
            [AuthorityValue::new(ConfigAuthority::User, ui.contrast)],
        )
        .unwrap_or_else(|_| unreachable!("the named user policy permits user values")),
        show_unavailable_choices: resolve_field(
            ShowUnavailableChoicesPolicy,
            PlayConfig::default().show_unavailable_choices,
            [AuthorityValue::new(
                ConfigAuthority::User,
                play.show_unavailable_choices,
            )],
        )
        .unwrap_or_else(|_| unreachable!("the named user policy permits user values")),
    }
}
