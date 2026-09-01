use super::UserConfigField;

/// Presence of each user-owned field after parsing or programmatic loading.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub(crate) struct UserConfigFieldPresence {
    pub(crate) ui_locale: bool,
    pub(crate) keymap: bool,
    pub(crate) key_hints: bool,
    pub(crate) color: bool,
    pub(crate) contrast: bool,
    pub(crate) show_unavailable_choices: bool,
}

impl UserConfigFieldPresence {
    pub(super) const fn all_explicit() -> Self {
        Self {
            ui_locale: true,
            keymap: true,
            key_hints: true,
            color: true,
            contrast: true,
            show_unavailable_choices: true,
        }
    }

    pub(super) const fn is_explicit(self, field: UserConfigField) -> bool {
        match field {
            UserConfigField::UiLocale => self.ui_locale,
            UserConfigField::Keymap => self.keymap,
            UserConfigField::KeyHints => self.key_hints,
            UserConfigField::Color => self.color,
            UserConfigField::Contrast => self.contrast,
            UserConfigField::ShowUnavailableChoices => self.show_unavailable_choices,
        }
    }
}
