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
    view::{Label, TextInput},
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
    /// An icon button's square hit area around its 20px face: as tall as the text buttons it
    /// replaced, never smaller than the comparison reference.
    pub const ICON_TARGET: f64 = 32.;
    /// The least hit target, after WCAG 2.2 AA §2.5.8, at every scale.
    pub const TARGET: f64 = 24.;
    /// A text control's padding either side of its words.
    pub const CONTROL_PAD: f64 = 12.;
    /// A label control's padding either side of its words: room for its 2px focus ring inside
    /// the edge, at every scale. It has no border.
    pub const LABEL_PAD: f64 = 4.;
    /// A field's padding and border before its text, at every scale.
    pub const FIELD_INSET: f64 = 2.;
    /// A search field's least width.
    pub const FIELD_MIN: f64 = 180.;
    /// The longest line of guidance and report prose on a sheet, at every scale.
    pub const MEASURE: f64 = 640.;
    /// The least width of the file tab, and of the wordmark's column while the scene list hides.
    pub const TAB_MIN: f64 = 96.;
    /// A picker's width, and how far below the window's top edge it opens.
    pub const PICKER_WIDTH: f64 = 640.;
    pub const PICKER_TOP: f64 = 72.;
    /// The most result lines a picker shows at once; the rest scroll.
    pub const PICKER_ROWS: usize = 8;
    /// The Settings side sheet's width.
    pub const SIDE_SHEET: f64 = 360.;
    /// The close decision slip's width.
    pub const DECISION_WIDTH: f64 = 560.;
    /// An anchored form slip's width, such as Configure preview.
    pub const SLIP_WIDTH: f64 = 480.;
    /// A master list's column beside its detail, such as the declarations.
    pub const MASTER_COLUMN: f64 = 320.;
    /// What a master column leaves to the window's chrome and its own search and count, when it
    /// fits its rows to the window's height.
    pub const MASTER_CHROME: f64 = 320.;
    /// A definition list's term column, such as a declaration's parameter names.
    pub const TERM_COLUMN: f64 = 160.;
}

/// A text control's padding and 1px border before its words, at `scale`: where a heading or
/// label beside such a control starts its text so the two line up.
pub(crate) fn control_inset(scale: f32) -> f64 {
    size::CONTROL_PAD * f64::from(scale) + 1.
}

/// A label control's padding before its words, at every scale: how far it hangs past its column
/// so the words keep the label's place.
pub(crate) fn label_inset() -> f64 {
    size::LABEL_PAD
}

/// The scene list's insets, before the UI scale.
pub(crate) mod sidebar {
    /// The list's scroll padding either side.
    pub const COLUMN: f64 = 24.;
    /// The scroll padding and a row's own padding either side.
    pub const ROW: f64 = 48.;
    /// Beats sit inside their scene, past its disclosure chevron.
    pub const OUTLINE: f64 = 12.;
}

/// Widths at which chrome changes form, before the UI scale.
pub(crate) mod breakpoint {
    /// The toolbar shows labels and the history buttons from this width.
    pub const TOOLBAR: f64 = 600.;
    /// Segments narrower than this stack.
    pub const SEGMENTS: f64 = 360.;
    /// A sheet's few stacked actions share a row from this width.
    pub const ACTIONS: f64 = 350.;
    /// Two equal columns share a sheet from this width, such as the queue and its entry.
    pub const TWO_COLUMNS: f64 = 600.;
    /// Two versions of a source sit side by side from this width.
    pub const COMPARISON: f64 = 920.;
    /// A field and its button share a row from this width.
    pub const FIELD_ROW: f64 = 480.;
    /// A banner's or a sheet head's actions share its row from this width.
    pub const SHEET_ROW: f64 = 640.;
    /// A master list and its detail sit side by side from this width.
    pub const MASTER_DETAIL: f64 = 720.;
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

/// Set an existing label or field in an interface role: family, size, weight and leading together.
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
/// A field's accessible bounds are its line box, so a field set in Body keeps a 20px line, taller
/// than the 16px text at 1.2 it replaced; no field shrinks from the reference.
impl<S: 'static, A: 'static> Typeset for TextInput<S, A> {
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
    /// Literata 400, 0.9R / 1.45: replies.
    Reply,
    /// Plex Mono at 0.6R: a reply's hanging number.
    Number,
    /// Plex Sans 600, 0.6R caps, +0.06em: speaker labels and "REPLIES" only. At least 11px.
    Label,
    /// Plex Sans 500, 0.72R: reply destinations. At least 13px.
    Destination,
}
impl Sheet {
    /// The reading size the sheet's interface roles are drawn for.
    pub const REFERENCE: f32 = 20.;
    /// The scale interface roles take on the sheet: the reading size over its reference, never
    /// the UI scale. The scene title (Display) is 1.4R, a beat heading (Section) 1.0R and the
    /// folio (Id) 0.6R, with their lines in proportion.
    pub fn scale(reading: f32) -> f32 {
        reading / Self::REFERENCE
    }
    /// The sheet scale for the running head and foot, whose Meta and Id text keep the 11px floor
    /// of the sheet's smallest type.
    pub fn folio_scale(reading: f32) -> f32 {
        Self::scale(reading).max(11. / Type::Meta.metrics().0)
    }
    /// The manuscript's leading, on editable fields and read-only text alike, so a beat sets the
    /// same lines whether or not its fields are mounted.
    pub const LEADING: LineHeight = LineHeight::FontSizeRelative(1.45);
    /// The role of a manuscript row's text: a reply when it has a reply number, a line otherwise.
    pub const fn text(reply: Option<usize>) -> Self {
        if reply.is_some() {
            Self::Reply
        } else {
            Self::Dialogue
        }
    }
    /// Size in logical pixels for reading size `reading`, floors included.
    pub fn size(self, reading: f32) -> f32 {
        match self {
            Self::Dialogue => reading,
            Self::Reply => 0.9 * reading,
            Self::Number => 0.6 * reading,
            Self::Label => (0.6 * reading).max(11.),
            Self::Destination => (0.72 * reading).max(13.),
        }
    }
    pub const fn weight(self) -> FontWeight {
        match self {
            Self::Dialogue | Self::Reply | Self::Number => FontWeight::NORMAL,
            Self::Label => FontWeight::SEMI_BOLD,
            Self::Destination => FontWeight::MEDIUM,
        }
    }
    pub const fn families(self) -> &'static [&'static str] {
        match self {
            Self::Dialogue | Self::Reply => FONT_PROSE,
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
