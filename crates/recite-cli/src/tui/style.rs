use ratatui::style::{Color, Modifier, Style};

use super::{TuiContrast, TuiSettings};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct TuiPalette {
    pub(crate) color_enabled: bool,
    pub(crate) contrast: TuiContrast,
}

impl Default for TuiPalette {
    fn default() -> Self {
        Self {
            color_enabled: true,
            contrast: TuiContrast::Standard,
        }
    }
}

impl TuiPalette {
    pub(crate) fn from_settings(settings: &TuiSettings) -> Self {
        Self {
            color_enabled: settings.color_enabled(),
            contrast: settings.contrast,
        }
    }

    pub(crate) fn line_label(self) -> Style {
        self.label()
    }

    pub(crate) fn prompt_label(self) -> Style {
        self.label()
    }

    pub(crate) fn choice_label(self) -> Style {
        self.label()
    }

    pub(crate) fn condition_label(self) -> Style {
        self.label()
    }

    pub(crate) fn effect_label(self) -> Style {
        self.label()
    }

    /// Labels, IDs and status: the second ink of the terminal.
    pub(crate) fn muted(self) -> Style {
        self.color(self.accessible(Color::DarkGray, Color::Gray))
    }

    pub(crate) fn title(self) -> Style {
        Style::default().add_modifier(Modifier::BOLD)
    }

    /// The one coloured thing on screen: what the player acts on next.
    pub(crate) fn selected_marker(self) -> Style {
        self.emphasis()
    }

    /// The selected row's marker and number carry the accent; other rows stay muted.
    pub(crate) fn choice_chrome(self, is_selected: bool, is_available: bool) -> Style {
        if is_selected && is_available {
            self.emphasis()
        } else {
            self.muted()
        }
    }

    pub(crate) fn emphasis(self) -> Style {
        self.color(self.accessible(Color::Magenta, Color::White))
            .add_modifier(Modifier::BOLD)
    }

    pub(crate) fn plain(self) -> Style {
        Style::default()
    }

    /// Every role label shares one muted ink; the words tell the roles apart.
    fn label(self) -> Style {
        self.muted().add_modifier(Modifier::BOLD)
    }

    fn accessible(self, standard: Color, accessible: Color) -> Color {
        match self.contrast {
            TuiContrast::Standard => standard,
            TuiContrast::Accessible => accessible,
        }
    }

    fn color(self, color: Color) -> Style {
        if self.color_enabled {
            Style::default().fg(color)
        } else {
            Style::default()
        }
    }
}
