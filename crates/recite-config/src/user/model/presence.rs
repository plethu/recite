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
    pub(crate) writer_confirm_exit: bool,
    pub(crate) writer_view: bool,
    pub(crate) writer_pane_side: bool,
    pub(crate) writer_theme: bool,
    pub(crate) writer_reduced_motion: bool,
    pub(crate) writer_zoom_to_pointer: bool,
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
            writer_confirm_exit: true,
            writer_view: true,
            writer_pane_side: true,
            writer_theme: true,
            writer_reduced_motion: true,
            writer_zoom_to_pointer: true,
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
            UserConfigField::WriterConfirmExit => self.writer_confirm_exit,
            UserConfigField::WriterView => self.writer_view,
            UserConfigField::WriterPaneSide => self.writer_pane_side,
            UserConfigField::WriterTheme => self.writer_theme,
            UserConfigField::WriterReducedMotion => self.writer_reduced_motion,
            UserConfigField::WriterZoomToPointer => self.writer_zoom_to_pointer,
        }
    }
}
