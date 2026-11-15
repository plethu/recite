//! The type and spacing scales. Every Writer gap, padding and text size comes from here, so
//! proximity encodes relationship and weight and size, not whitespace, carry hierarchy.
//!
//! Interface roles multiply by the UI scale. Sheet roles derive from the reading size, never from
//! the UI scale, and keep a floor so small reading sizes stay legible.
#[cfg(test)]
mod tests;
use super::{label, print::font};
use recite_writer_session::typography::{FONT_DISPLAY, FONT_MONO, FONT_PROSE, FONT_UI};
use xilem::{
    masonry::{
        core::ArcStr,
        parley::{LineHeight, style::FontWeight},
    },
    view::Label,
};

/// The spacing scale: 4, 8, 12, 16, 24, 32 and 48. One and two pixels are borders and rings only.
pub(crate) mod space {
    /// Inside one unit: label and field, reply and destination, line and source id, icon and text.
    pub const UNIT: f64 = 4.;
    /// Interface siblings: rows, buttons, menu items and tabs.
    pub const SIBLINGS: f64 = 8.;
    /// Text siblings: line and line, reply and reply.
    pub const TEXT: f64 = 12.;
    /// Different kinds within a group, and the padding inside every pane.
    pub const KINDS: f64 = 16.;
    /// Groups: a dialogue block and its replies, or two groups of a form.
    pub const GROUPS: f64 = 24.;
    /// Regions and sections.
    pub const REGIONS: f64 = 32.;
    /// Beat to beat, with a rule above the beat heading; the widest region gap.
    pub const BEATS: f64 = 48.;
}

/// Control and row geometry, before the UI scale.
pub(crate) mod size {
    /// A text control on a page or sheet, and a list row. The agreed 32 rounded up past the
    /// comparison reference's 32.2, so no hit target shrinks.
    pub const ROW: f64 = 33.;
}

/// Text in a single-line field. A text input's accessible bounds are its line box, so fields
/// keep 16px text until inputs take a line height of their own; none shrinks from the reference.
pub(crate) fn field_text(scale: f32) -> f32 {
    16. * scale
}

/// An interface type role.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Type {
    /// Fraunces 600, 28/32: the scene title and page titles.
    Display,
    /// Fraunces 600, 20/24: beat headings, panel and tool sections.
    Section,
    /// Plex Sans 500, 14/20: buttons, tabs, rows and menu items.
    Control,
    /// Plex Sans 400, 14/20: guidance and sentences.
    Body,
    /// Plex Sans 400, 12/16, muted by the caller: counts, hints and status sentences.
    Meta,
    /// Plex Mono 12/16: ids, source labels and the folio only.
    Id,
}
impl Type {
    /// Size and line height in logical pixels at 100% UI scale.
    pub const fn metrics(self) -> (f32, f32) {
        match self {
            Self::Display => (28., 32.),
            Self::Section => (20., 24.),
            Self::Control | Self::Body => (14., 20.),
            Self::Meta | Self::Id => (12., 16.),
        }
    }
    pub const fn weight(self) -> FontWeight {
        match self {
            Self::Display | Self::Section => FontWeight::SEMI_BOLD,
            Self::Control => FontWeight::MEDIUM,
            Self::Body | Self::Meta | Self::Id => FontWeight::NORMAL,
        }
    }
    pub const fn families(self) -> &'static [&'static str] {
        match self {
            Self::Display | Self::Section => FONT_DISPLAY,
            Self::Control | Self::Body | Self::Meta => FONT_UI,
            Self::Id => FONT_MONO,
        }
    }
    pub fn size(self, scale: f32) -> f32 {
        self.metrics().0 * scale
    }
    /// A label set in this role at `scale`.
    pub fn label(self, scale: f32, text: impl Into<ArcStr>) -> Label {
        label(scale, text).typeset(self, scale)
    }
}

/// Set an existing label in an interface role: family, size, weight and leading together.
pub(crate) trait Typeset {
    fn typeset(self, role: Type, scale: f32) -> Self;
}
impl Typeset for Label {
    fn typeset(self, role: Type, scale: f32) -> Self {
        let (size, line) = role.metrics();
        self.font(font(role.families()))
            .text_size(size * scale)
            .weight(role.weight())
            .line_height(LineHeight::Absolute(line * scale))
    }
}

/// A sheet type role, derived from the reading size R.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Sheet {
    /// Literata 400, R / 1.45R: lines.
    Dialogue,
    /// Plex Mono at 0.6R: a reply's hanging number.
    Number,
    /// Plex Sans 600, 0.6R caps, +0.06em: speaker labels and "REPLIES" only. At least 11px.
    Label,
    /// Plex Sans 500, 0.72R: reply destinations. At least 13px.
    Destination,
}
impl Sheet {
    /// Size in logical pixels for reading size `reading`, floors included.
    pub fn size(self, reading: f32) -> f32 {
        match self {
            Self::Dialogue => reading,
            Self::Number => 0.6 * reading,
            Self::Label => (0.6 * reading).max(11.),
            Self::Destination => (0.72 * reading).max(13.),
        }
    }
    pub const fn weight(self) -> FontWeight {
        match self {
            Self::Dialogue | Self::Number => FontWeight::NORMAL,
            Self::Label => FontWeight::SEMI_BOLD,
            Self::Destination => FontWeight::MEDIUM,
        }
    }
    pub const fn families(self) -> &'static [&'static str] {
        match self {
            Self::Dialogue => FONT_PROSE,
            Self::Number => FONT_MONO,
            Self::Label | Self::Destination => FONT_UI,
        }
    }
    /// Tracking for caps labels: +0.06em.
    pub fn tracking(self, reading: f32) -> f32 {
        if self == Self::Label {
            0.06 * self.size(reading)
        } else {
            0.
        }
    }
    /// A label set in this role for reading size `reading`.
    pub fn label(self, reading: f32, text: impl Into<ArcStr>) -> Label {
        label(1., text)
            .font(font(self.families()))
            .text_size(self.size(reading))
            .weight(self.weight())
            .letter_spacing(self.tracking(reading))
    }
}
