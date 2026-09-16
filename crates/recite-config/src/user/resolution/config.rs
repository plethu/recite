use super::super::LoadedUserConfig;
use super::super::model::{
    ConfigAuthority, KeyHints, Keymap, PlayConfig, TuiColorMode, TuiContrast, UserConfigField,
};
use super::policy::{
    AuthorityValue, ColorPolicy, ContrastPolicy, KeyHintsPolicy, KeymapPolicy, ResolvedField,
    ShowUnavailableChoicesPolicy, UiLocalePolicy, WriterConfirmExitPolicy, resolve_field,
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
    writer_confirm_exit: ResolvedField<bool>,
    writer_view: ResolvedField<super::super::WriterView>,
    writer_theme: ResolvedField<super::super::WriterTheme>,
    writer_reduced_motion: ResolvedField<bool>,
    writer_zoom_to_pointer: ResolvedField<bool>,
}

impl ResolvedUserConfig {
    pub const fn writer_view(&self) -> &ResolvedField<super::super::WriterView> {
        &self.writer_view
    }
    pub const fn writer_theme(&self) -> &ResolvedField<super::super::WriterTheme> {
        &self.writer_theme
    }
    pub const fn writer_reduced_motion(&self) -> &ResolvedField<bool> {
        &self.writer_reduced_motion
    }
    pub const fn writer_zoom_to_pointer(&self) -> &ResolvedField<bool> {
        &self.writer_zoom_to_pointer
    }

    /// Whether to confirm ordinary native writer exits, with user/default provenance.
    #[must_use]
    pub const fn writer_confirm_exit(&self) -> &ResolvedField<bool> {
        &self.writer_confirm_exit
    }

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
    loaded: &LoadedUserConfig,
    invocation: &InvocationOverrides,
) -> ResolvedUserConfig {
    let ui = &loaded.config.ui;
    let play = &loaded.config.play;
    ResolvedUserConfig {
        writer_view: resolve_field(
            super::policy::WriterViewPolicy,
            super::super::WriterConfig::default().view,
            loaded
                .field_is_explicit(UserConfigField::WriterView)
                .then(|| AuthorityValue::new(ConfigAuthority::User, loaded.config.writer.view)),
        )
        .unwrap_or_else(|_| unreachable!("writer preferences are user owned")),
        writer_theme: resolve_field(
            super::policy::WriterThemePolicy,
            super::super::WriterConfig::default().theme,
            loaded
                .field_is_explicit(UserConfigField::WriterTheme)
                .then(|| AuthorityValue::new(ConfigAuthority::User, loaded.config.writer.theme)),
        )
        .unwrap_or_else(|_| unreachable!("writer preferences are user owned")),
        writer_reduced_motion: resolve_field(
            super::policy::WriterReducedMotionPolicy,
            super::super::WriterConfig::default().reduced_motion,
            loaded
                .field_is_explicit(UserConfigField::WriterReducedMotion)
                .then(|| {
                    AuthorityValue::new(ConfigAuthority::User, loaded.config.writer.reduced_motion)
                }),
        )
        .unwrap_or_else(|_| unreachable!("writer preferences are user owned")),
        writer_zoom_to_pointer: resolve_field(
            super::policy::WriterZoomToPointerPolicy,
            super::super::WriterConfig::default().zoom_to_pointer,
            loaded
                .field_is_explicit(UserConfigField::WriterZoomToPointer)
                .then(|| {
                    AuthorityValue::new(ConfigAuthority::User, loaded.config.writer.zoom_to_pointer)
                }),
        )
        .unwrap_or_else(|_| unreachable!("writer preferences are user owned")),

        writer_confirm_exit: resolve_field(
            WriterConfirmExitPolicy,
            super::super::WriterConfig::default().confirm_exit,
            loaded
                .field_is_explicit(UserConfigField::WriterConfirmExit)
                .then(|| {
                    AuthorityValue::new(ConfigAuthority::User, loaded.config.writer.confirm_exit)
                }),
        )
        .unwrap_or_else(|_| unreachable!("writer exit confirmation is a user preference")),
        ui_locale: resolve_field(
            UiLocalePolicy,
            UiLocale::default(),
            [loaded
                .field_is_explicit(UserConfigField::UiLocale)
                .then(|| AuthorityValue::new(ConfigAuthority::User, ui.locale.clone()))]
            .into_iter()
            .flatten(),
        )
        .unwrap_or_else(|_| unreachable!("the named user policy permits user values")),
        keymap: resolve_field(
            KeymapPolicy,
            Keymap::default(),
            [loaded
                .field_is_explicit(UserConfigField::Keymap)
                .then(|| AuthorityValue::new(ConfigAuthority::User, ui.keymap))]
            .into_iter()
            .flatten()
            .chain(
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
            [loaded
                .field_is_explicit(UserConfigField::KeyHints)
                .then(|| AuthorityValue::new(ConfigAuthority::User, ui.key_hints))]
            .into_iter()
            .flatten(),
        )
        .unwrap_or_else(|_| unreachable!("the named user policy permits user values")),
        color: resolve_field(
            ColorPolicy,
            TuiColorMode::default(),
            [loaded
                .field_is_explicit(UserConfigField::Color)
                .then(|| AuthorityValue::new(ConfigAuthority::User, ui.color))]
            .into_iter()
            .flatten(),
        )
        .unwrap_or_else(|_| unreachable!("the named user policy permits user values")),
        contrast: resolve_field(
            ContrastPolicy,
            TuiContrast::default(),
            [loaded
                .field_is_explicit(UserConfigField::Contrast)
                .then(|| AuthorityValue::new(ConfigAuthority::User, ui.contrast))]
            .into_iter()
            .flatten(),
        )
        .unwrap_or_else(|_| unreachable!("the named user policy permits user values")),
        show_unavailable_choices: resolve_field(
            ShowUnavailableChoicesPolicy,
            PlayConfig::default().show_unavailable_choices,
            [loaded
                .field_is_explicit(UserConfigField::ShowUnavailableChoices)
                .then(|| {
                    AuthorityValue::new(ConfigAuthority::User, play.show_unavailable_choices)
                })]
            .into_iter()
            .flatten(),
        )
        .unwrap_or_else(|_| unreachable!("the named user policy permits user values")),
    }
}
