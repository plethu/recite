//! The sheet's type: roles derived from the reading size R, zoomed by the UI scale as a browser
//! zooms a page. The Reading size setting is the sheet's size at 100%; at 200% every sheet role,
//! its floors and the grid's columns double, and the text reflows inside the pane.
use super::{super::label, Type, size};
use crate::presentation::print::font;
use recite_writer_app::Writer;
use recite_writer_session::typography::{FONT_MONO, FONT_PROSE, FONT_UI};
use xilem::{
    masonry::{
        core::ArcStr,
        parley::{LineHeight, style::FontWeight},
    },
    view::Label,
};

/// The sheet's size: the reading size R and the UI scale that zooms it.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Page {
    /// The Reading size setting: R at 100%.
    pub reading: f32,
    /// The UI scale.
    pub zoom: f32,
}
impl Page {
    /// The reference sheet: R at its reference size, at 100%.
    pub const REFERENCE: Self = Self::new(Sheet::REFERENCE, 1.);
    pub const fn new(reading: f32, zoom: f32) -> Self {
        Self { reading, zoom }
    }
    /// The sheet `state` sets: its reading size, zoomed by its UI scale.
    pub fn of(state: &Writer) -> Self {
        Self::new(state.reading_size(), state.ui_scale())
    }
    /// The scale interface roles take on the sheet: the reading size over its reference, zoomed.
    /// The scene title takes [`Self::title`] and beat headings [`Self::section`].
    pub fn scale(self) -> f32 {
        self.reading / Sheet::REFERENCE * self.zoom
    }
    /// The scene title's scale: Display at 1.6R, a clear step above the beat headings under it,
    /// and never smaller than every other title.
    pub fn title(self) -> f32 {
        (1.6 * self.reading / Type::Display.metrics().0).max(1.) * self.zoom
    }
    /// A beat heading's scale: Section at 1.2R, one step above the 1.0R lines it heads.
    pub fn section(self) -> f32 {
        1.2 * self.reading / Type::Section.metrics().0 * self.zoom
    }
    /// The running head's and foot's scale, whose Meta and Id text keep the sheet's 11px floor,
    /// zoomed with it.
    pub fn folio(self) -> f32 {
        self.scale().max(11. * self.zoom / Type::Meta.metrics().0)
    }
    /// The scale of controls set on the sheet: they zoom with it and never fall below the
    /// interface's own size, so no target shrinks from its reference at a small reading size.
    pub fn control(self) -> f32 {
        self.scale().max(self.zoom)
    }
    /// The least hit target on the sheet.
    pub fn target(self) -> f64 {
        size::TARGET * f64::from(self.control())
    }
}

/// A sheet type role, derived from the reading size R and zoomed by the UI scale.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub(crate) enum Sheet {
    /// Literata 400, R / 1.45R: lines.
    Dialogue,
    /// Literata 400, 0.9R / 1.45: replies.
    Reply,
    /// Plex Mono at 0.6R: a reply's hanging number and a line's id. At least 11px.
    Number,
    /// Plex Sans 600, 0.6R caps, +0.06em: speaker labels and "REPLIES" only. At least 11px.
    Label,
    /// Plex Sans 500, 0.72R: reply destinations. At least 13px.
    Destination,
}
impl Sheet {
    /// The reading size the sheet's interface roles are drawn for.
    pub const REFERENCE: f32 = 20.;
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
    /// Size in logical pixels on `page`: the role at its reading size, floors included, zoomed.
    pub fn size(self, page: Page) -> f32 {
        let reading = page.reading;
        page.zoom
            * match self {
                Self::Dialogue => reading,
                Self::Reply => 0.9 * reading,
                Self::Number | Self::Label => (0.6 * reading).max(11.),
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
    pub fn tracking(self, page: Page) -> f32 {
        if self == Self::Label {
            0.06 * self.size(page)
        } else {
            0.
        }
    }
    /// A label set in this role on `page`.
    pub fn label(self, page: Page, text: impl Into<ArcStr>) -> Label {
        label(1., text)
            .font(font(self.families()))
            .text_size(self.size(page))
            .weight(self.weight())
            .letter_spacing(self.tracking(page))
    }
}
