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
        self.label(TuiColorRole::Line)
    }

    pub(crate) fn prompt_label(self) -> Style {
        self.label(TuiColorRole::Prompt)
    }

    pub(crate) fn choice_label(self) -> Style {
        self.label(TuiColorRole::Choice)
    }

    pub(crate) fn condition_label(self) -> Style {
        self.label(TuiColorRole::Condition)
    }

    pub(crate) fn effect_label(self) -> Style {
        self.label(TuiColorRole::Effect)
    }

    pub(crate) fn muted(self) -> Style {
        self.color(Color::DarkGray)
    }

    pub(crate) fn title(self) -> Style {
        self.color(Color::Cyan).add_modifier(Modifier::BOLD)
    }

    pub(crate) fn selected_marker(self) -> Style {
        self.color(self.accessible(Color::Yellow, Color::White))
            .add_modifier(Modifier::BOLD)
    }

    pub(crate) fn choice_chrome(self, is_selected: bool, is_available: bool) -> Style {
        let style = if is_available {
            self.color(self.accessible(Color::Cyan, Color::White))
        } else {
            self.muted()
        };
        if is_selected {
            style.add_modifier(Modifier::BOLD)
        } else {
            style
        }
    }

    pub(crate) fn emphasis(self) -> Style {
        self.color(self.accessible(Color::Magenta, Color::White))
            .add_modifier(Modifier::BOLD)
    }

    pub(crate) fn plain(self) -> Style {
        Style::default()
    }

    fn label(self, role: TuiColorRole) -> Style {
        self.color(match (self.contrast, role) {
            (TuiContrast::Standard, TuiColorRole::Line) => Color::Green,
            (TuiContrast::Standard, TuiColorRole::Prompt) => Color::Blue,
            (TuiContrast::Standard, TuiColorRole::Choice) => Color::Cyan,
            (TuiContrast::Standard, TuiColorRole::Condition) => Color::Yellow,
            (TuiContrast::Standard, TuiColorRole::Effect) => Color::Magenta,
            (TuiContrast::Accessible, TuiColorRole::Line) => Color::LightGreen,
            (TuiContrast::Accessible, TuiColorRole::Prompt) => Color::LightBlue,
            (TuiContrast::Accessible, TuiColorRole::Choice) => Color::White,
            (TuiContrast::Accessible, TuiColorRole::Condition) => Color::LightYellow,
            (TuiContrast::Accessible, TuiColorRole::Effect) => Color::LightMagenta,
        })
        .add_modifier(Modifier::BOLD)
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

#[derive(Clone, Copy)]
enum TuiColorRole {
    Line,
    Prompt,
    Choice,
    Condition,
    Effect,
}
