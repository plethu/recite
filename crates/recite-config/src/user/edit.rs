//! Typed, source-preserving edits to user-owned settings.
use super::{KeyHints, Keymap, TuiColorMode, TuiContrast, WriterPaneSide, WriterTheme, WriterView};
use recite_ui::UiLocale;
use toml_edit::{DocumentMut, Value};

/// One explicit preference change. Unmentioned fields retain their source text.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum UserConfigEdit {
    UiLocale(UiLocale),
    Keymap(Keymap),
    KeyHints(KeyHints),
    Color(TuiColorMode),
    Contrast(TuiContrast),
    ShowUnavailableChoices(bool),
    WriterConfirmExit(bool),
    WriterView(WriterView),
    WriterPaneSide(WriterPaneSide),
    WriterTheme(WriterTheme),
    WriterReducedMotion(bool),
    WriterZoomToPointer(bool),
}

impl UserConfigEdit {
    /// Apply an edit to an in-memory preference draft. Persistence still validates
    /// and merges against the current file through UserConfigStore.
    pub fn apply_to(&self, config: &mut super::UserConfig) {
        match self {
            Self::UiLocale(v) => config.ui.locale = v.clone(),
            Self::Keymap(v) => config.ui.keymap = *v,
            Self::KeyHints(v) => config.ui.key_hints = *v,
            Self::Color(v) => config.ui.color = *v,
            Self::Contrast(v) => config.ui.contrast = *v,
            Self::ShowUnavailableChoices(v) => config.play.show_unavailable_choices = *v,
            Self::WriterConfirmExit(v) => config.writer.confirm_exit = *v,
            Self::WriterView(v) => config.writer.view = *v,
            Self::WriterPaneSide(v) => config.writer.pane_side = *v,
            Self::WriterTheme(v) => config.writer.theme = *v,
            Self::WriterReducedMotion(v) => config.writer.reduced_motion = *v,
            Self::WriterZoomToPointer(v) => config.writer.zoom_to_pointer = *v,
        }
    }

    pub(super) fn apply(&self, document: &mut DocumentMut) {
        let (section, field, value): (_, _, Value) = match self {
            Self::UiLocale(locale) => ("ui", "locale", locale.to_string().into()),
            Self::Keymap(keymap) => (
                "ui",
                "keymap",
                match keymap {
                    Keymap::Standard => "standard",
                    Keymap::Vim => "vim",
                }
                .into(),
            ),
            Self::KeyHints(hints) => (
                "ui",
                "key_hints",
                match hints {
                    KeyHints::Contextual => "contextual",
                    KeyHints::Compact => "compact",
                    KeyHints::Hidden => "hidden",
                }
                .into(),
            ),
            Self::Color(color) => (
                "ui",
                "color",
                match color {
                    TuiColorMode::Auto => "auto",
                    TuiColorMode::Always => "always",
                    TuiColorMode::Never => "never",
                }
                .into(),
            ),
            Self::Contrast(contrast) => (
                "ui",
                "contrast",
                match contrast {
                    TuiContrast::Standard => "standard",
                    TuiContrast::Accessible => "accessible",
                }
                .into(),
            ),
            Self::ShowUnavailableChoices(show) => {
                ("play", "show_unavailable_choices", (*show).into())
            }
            Self::WriterView(view) => (
                "writer",
                "view",
                match view {
                    WriterView::Map => "map",
                    WriterView::Source => "source",
                }
                .into(),
            ),
            Self::WriterPaneSide(pane_side) => (
                "writer",
                "pane_side",
                match pane_side {
                    WriterPaneSide::Left => "left",
                    WriterPaneSide::Right => "right",
                }
                .into(),
            ),
            Self::WriterTheme(theme) => (
                "writer",
                "theme",
                match theme {
                    WriterTheme::Light => "light",
                    WriterTheme::Dark => "dark",
                }
                .into(),
            ),
            Self::WriterReducedMotion(value) => ("writer", "reduced_motion", (*value).into()),
            Self::WriterZoomToPointer(value) => ("writer", "zoom_to_pointer", (*value).into()),
            Self::WriterConfirmExit(confirm) => ("writer", "confirm_exit", (*confirm).into()),
        };
        let slot = &mut document[section][field];
        let mut value = value;
        if let Some(previous) = slot.as_value() {
            *value.decor_mut() = previous.decor().clone();
        }
        *slot = toml_edit::Item::Value(value);
    }
}
