//! Personal preferences for the native writer.
use serde::Deserialize;

/// Native writer preferences stored in the user's Recite configuration.
#[derive(Clone, Debug, Eq, PartialEq, Deserialize)]
#[serde(default, deny_unknown_fields)]
pub struct WriterConfig {
    /// Ask before an ordinary exit. Unsaved-work protection remains mandatory.
    pub confirm_exit: bool,
    pub view: WriterView,
    pub pane_side: WriterPaneSide,
    pub theme: WriterTheme,
    pub reduced_motion: bool,
    pub zoom_to_pointer: bool,
}

impl Default for WriterConfig {
    fn default() -> Self {
        Self {
            confirm_exit: true,
            view: WriterView::Map,
            pane_side: WriterPaneSide::Right,
            theme: WriterTheme::Light,
            reduced_motion: false,
            zoom_to_pointer: true,
        }
    }
}

/// Preferred writer workspace, independent of a scene's editing selection.
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq, Deserialize)]
#[serde(rename_all = "lowercase")]
pub enum WriterView {
    #[default]
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
