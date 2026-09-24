//! Personal preferences for the native writer.
use serde::Deserialize;

/// Native writer preferences stored in the user's Recite configuration.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WriterConfig {
    /// Ask before an ordinary exit. Unsaved-work protection remains mandatory.
    pub confirm_exit: bool,
    pub presentation: WriterPresentation,
    pub view: WriterView,
    pub pane_side: WriterPaneSide,
    pub theme: WriterTheme,
    pub reduced_motion: bool,
    pub monochrome: bool,
    pub shortcut_hints: bool,
    pub shortcuts: WriterShortcuts,
    pub zoom_to_pointer: bool,
}

impl Default for WriterConfig {
    fn default() -> Self {
        Self {
            confirm_exit: true,
            presentation: WriterPresentation::default(),
            view: WriterView::Script,
            pane_side: WriterPaneSide::Right,
            theme: WriterTheme::Light,
            reduced_motion: false,
            monochrome: false,
            shortcut_hints: false,
            shortcuts: WriterShortcuts::default(),
            zoom_to_pointer: true,
        }
    }
}

/// Preferred writer workspace, independent of a scene's editing selection.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WriterView {
    #[default]
    Script,
    Map,
    Source,
}

/// Which side of the map holds the script editing pane.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WriterPaneSide {
    Left,
    #[default]
    Right,
}

/// Writer colour scheme; independent of terminal colour policy.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WriterTheme {
    #[default]
    Light,
    Dark,
}

mod presentation;
pub use presentation::{WriterPresentation, WriterPresentationError, WriterPresentationField};

mod shortcuts;
pub use shortcuts::{ShortcutError, WriterCommand, WriterShortcut, WriterShortcuts};
