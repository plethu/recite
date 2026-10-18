//! Visual treatment of control states; activation and semantics stay with Button.
use super::{Button, Kind};
use crate::design::{palette::Palette, tokens as t};
use freya::prelude::*;

/// Which filled ink a control shows: plum for the one primary action, ink for switched-on state.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum Fill {
    None,
    Accent,
    Ink,
}

impl Button {
    pub(super) fn fill(&self) -> Fill {
        match self.kind {
            Kind::Primary => Fill::Accent,
            Kind::Secondary if self.highlighted => Fill::Ink,
            _ if self.segment && self.highlighted => Fill::Ink,
            _ => Fill::None,
        }
    }

    /// Quiet list rows show selection as the second-ink plate with a bar, not a grey fill.
    pub(super) fn plated(&self) -> bool {
        self.kind == Kind::Quiet
            && self.highlighted
            && !self.segment
            && !self.semantics.is_tab()
            && self.enabled
    }

    /// The active document tab is marked by a second-ink rule along its lower edge.
    pub(super) fn underlined(&self) -> bool {
        self.semantics.is_tab() && self.highlighted && self.enabled
    }

    pub(super) fn background(&self, colors: &Palette, hovered: bool, pressed: bool) -> Color {
        let fill = self.fill();
        if !self.enabled {
            match self.kind {
                Kind::Primary | Kind::Field => colors.inset,
                _ => Color::TRANSPARENT,
            }
        } else if self.segment {
            // The group's travelling indicator is the selected segment's fill.
            if self.highlighted {
                Color::TRANSPARENT
            } else if pressed {
                colors.pressed
            } else if hovered {
                colors.hover
            } else {
                Color::TRANSPARENT
            }
        } else {
            let lift = if pressed {
                t::STAMP_PRESSED
            } else if hovered {
                t::STAMP_HOVER
            } else {
                0.
            };
            match fill {
                Fill::Accent => Color::lerp(colors.accent, colors.on_accent, lift * 0.7),
                Fill::Ink => colors.stamp_state(lift),
                Fill::None if pressed => colors.pressed,
                // The thumb-index plate is drawn as its own layer so it can bleed past the row.
                Fill::None if self.plated() => Color::TRANSPARENT,
                Fill::None if hovered => colors.hover,
                Fill::None if self.kind == Kind::Field => colors.inset,
                Fill::None => Color::TRANSPARENT,
            }
        }
    }

    pub(super) fn border_color(&self, colors: &Palette, focused: bool, stamped: bool) -> Color {
        if focused {
            match self.fill() {
                Fill::Accent => colors.on_accent,
                Fill::Ink => colors.on_stamp,
                Fill::None => colors.accent,
            }
        } else if self.kind == Kind::Field {
            colors.boundary
        } else if stamped && self.kind == Kind::Secondary {
            if self.enabled {
                colors.stamp
            } else {
                colors.rule
            }
        } else {
            Color::TRANSPARENT
        }
    }

    pub(super) fn border_width(&self, focused: bool, stamped: bool) -> f32 {
        if focused {
            t::FOCUS_WIDTH
        } else if stamped && self.kind == Kind::Secondary {
            t::STROKE_WIDTH
        } else {
            1.
        }
    }

    pub(super) fn text_color(&self, colors: &Palette, pressed: bool) -> Color {
        if !self.enabled {
            return colors.muted;
        }
        match self.fill() {
            Fill::Accent => colors.on_accent,
            Fill::Ink => colors.on_stamp,
            Fill::None if self.destination => colors.destination(pressed),
            Fill::None => colors.ink,
        }
    }
}
