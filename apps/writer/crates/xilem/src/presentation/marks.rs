//! Printer's marks: one family, painted around the work and never on its words, the same in the
//! galley, the lists and on map cards. They differ in shape as well as ink, so they hold without
//! hue: crop marks at the corners of the current beat, a halftone dot for unsaved work ("halftone
//! means not yet fixed"), an ink underscore under a selected row's words, and a focusable error
//! wedge for a problem. None of them takes layout room, input or an accessible name.
#[cfg(test)]
mod tests;
mod wedge;
use super::material::{Decorated, Decoration};
use recite_writer_session::palette::Palette;
pub(crate) use wedge::{Wedge, wedge, wedge_glyph};
use xilem::{
    WidgetView,
    masonry::{
        imaging::Painter,
        kurbo::{Circle, Rect},
        layout::AsUnit,
    },
    peniko::Color,
    style::Style,
    view::{flex_col, sized_box},
};

/// A decorative wedge's side, where a row or card repeats a problem its name already gives.
pub(crate) const GLYPH: f64 = 16.;
/// The unsaved dot's diameter at 100%.
pub(crate) const DOT: f64 = 8.;
/// How far out from a beat's leading edge the galley's unsaved dot is centred, at 100%.
const DOT_REACH: f64 = 12.;
/// Clear paper every mark leaves between itself and the edge of the room it sits in.
const CLEAR: f64 = 4.;

/// Crop marks: at each corner of a block, a pair of ticks along its edges' lines, detached from the
/// corner and pointing outward, as trim marks on a press sheet.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Ticks {
    /// Distance from the corner to each tick.
    pub gap: f64,
    /// Each tick's length.
    pub arm: f64,
    /// Each tick's thickness.
    pub weight: f64,
    /// Stock beyond the block's trailing and bottom edges, such as a card's offset, which the
    /// marks clear as part of the block.
    pub stock: f64,
    /// The block's width where it differs from its box: a beat is as wide as its grid, though a
    /// route hanging in the destination column reaches past it.
    pub span: Option<f64>,
}
impl Ticks {
    /// Ticks at `scale` for a block with `room` of clear paper around it: 2×8 ticks 4px off the
    /// corner on a sheet, or 6px ticks 2px off on a full-bleed page, never reaching closer than
    /// 4px to the room's edge.
    pub fn new(scale: f64, room: f64, sheet: bool) -> Self {
        let (gap, arm) = if sheet { (4., 8.) } else { (2., 6.) };
        let (gap, arm, weight) = (gap * scale, arm * scale, 2. * scale);
        Self {
            gap,
            arm: arm.min(room - CLEAR - gap).max(weight),
            weight,
            stock: 0.,
            span: None,
        }
    }
    /// The same ticks clearing `stock` beyond the trailing and bottom edges.
    pub fn clearing(self, stock: f64) -> Self {
        Self { stock, ..self }
    }
    /// The same ticks around a block `span` wide, whatever its box.
    pub fn spanning(self, span: f64) -> Self {
        Self {
            span: Some(span),
            ..self
        }
    }
    /// The eight ticks around `block`.
    pub fn rects(self, block: Rect) -> [Rect; 8] {
        let Self {
            gap,
            arm,
            weight,
            stock,
            span,
        } = self;
        let width = span.unwrap_or(block.width());
        let (x0, y0, x1, y1) = (
            block.x0,
            block.y0,
            block.x0 + width + stock,
            block.y1 + stock,
        );
        [
            // Top leading: across, then up.
            Rect::new(x0 - gap - arm, y0, x0 - gap, y0 + weight),
            Rect::new(x0, y0 - gap - arm, x0 + weight, y0 - gap),
            // Top trailing.
            Rect::new(x1 + gap, y0, x1 + gap + arm, y0 + weight),
            Rect::new(x1 - weight, y0 - gap - arm, x1, y0 - gap),
            // Bottom leading.
            Rect::new(x0 - gap - arm, y1 - weight, x0 - gap, y1),
            Rect::new(x0, y1 + gap, x0 + weight, y1 + gap + arm),
            // Bottom trailing.
            Rect::new(x1 + gap, y1 - weight, x1 + gap + arm, y1),
            Rect::new(x1 - weight, y1 + gap, x1, y1 + gap + arm),
        ]
    }
}

/// Where the unsaved dot sits relative to the child it marks.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) enum DotAt {
    /// Centred `reach` out from the leading edge, sitting on the first baseline: a line's draft.
    Baseline(f64),
    /// Centred `reach` out from the leading edge, halfway down: a heading's draft.
    Beside(f64),
    /// At the centre of the child: a slot reserved for it.
    Centre,
    /// At a point in the child's own coordinates: a card's corner.
    At(f64, f64),
}

/// The unsaved dot: a solid disc, the enlarged halftone dot and the editor tab's "modified" dot.
#[derive(Clone, Copy, Debug, PartialEq)]
pub(crate) struct Dot {
    pub radius: f64,
    pub at: DotAt,
}
impl Dot {
    /// The dot at `scale`, placed `at`.
    pub fn new(scale: f64, at: DotAt) -> Self {
        Self {
            radius: DOT / 2. * scale,
            at,
        }
    }
    /// The galley's dot at `scale`: in the sheet's pad, `room` wide, 12px out from the beat's
    /// leading edge, clear of the pad's own edge and of the margin column. A pad too narrow for
    /// the doubled dot, as on a full-bleed page at 200%, keeps the 100% dot. Rows nested `inset`
    /// into the beat keep it beside the beat's own edge.
    pub fn galley(scale: f64, room: f64, inset: f64, baseline: bool) -> Self {
        let radius = (DOT * scale).min(room - CLEAR - 2. * scale).max(DOT) / 2.;
        let reach = (DOT_REACH * scale).min(room - CLEAR - radius) + inset;
        Self {
            radius,
            at: if baseline {
                DotAt::Baseline(reach)
            } else {
                DotAt::Beside(reach)
            },
        }
    }
    /// The disc beside or inside `bounds`, whose first baseline lies `baseline` below its top.
    pub fn circle(self, bounds: Rect, baseline: f64) -> Circle {
        let centre = match self.at {
            DotAt::Baseline(reach) => (bounds.x0 - reach, bounds.y0 + baseline - self.radius),
            DotAt::Beside(reach) => (bounds.x0 - reach, bounds.center().y),
            DotAt::Centre => bounds.center().into(),
            DotAt::At(x, y) => (bounds.x0 + x, bounds.y0 + y),
        };
        Circle::new(centre, self.radius)
    }
}

/// The marks painted around one child; each is absent while `None`.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub(crate) struct Marks {
    pub ticks: Option<(Color, Ticks)>,
    pub dot: Option<(Color, Dot)>,
}
impl Marks {
    /// The current beat's crop marks in the palette's crop ink, while `current`.
    pub fn crop(palette: &Palette<Color>, current: bool, ticks: Ticks) -> Self {
        Self {
            ticks: current.then_some((palette.crop, ticks)),
            dot: None,
        }
    }
    /// These marks and the unsaved dot, while `unsaved`.
    pub fn unsaved(self, palette: &Palette<Color>, unsaved: bool, dot: Dot) -> Self {
        Self {
            dot: unsaved.then_some((palette.unsaved, dot)),
            ..self
        }
    }
    /// Paint the marks around `bounds`, whose first baseline lies `baseline` below its top.
    pub fn paint(&self, painter: &mut Painter<'_>, bounds: Rect, baseline: f64) {
        if let Some((color, ticks)) = self.ticks {
            for tick in ticks.rects(bounds) {
                painter.fill(tick, color).draw();
            }
        }
        if let Some((color, dot)) = self.dot {
            painter.fill(dot.circle(bounds, baseline), color).draw();
        }
    }
}

/// `child` with `marks` painted around it.
pub(crate) fn marked<V>(marks: Marks, child: V) -> Decorated<V> {
    Decorated::new(Decoration::Marks(marks), child)
}

/// The slot a list row keeps for the unsaved dot at its trailing end, `height` tall, reserved on
/// every row so nothing moves when the dot appears.
pub(crate) fn dot_slot<S: 'static>(
    palette: &Palette<Color>,
    scale: f64,
    unsaved: bool,
    height: f64,
) -> impl WidgetView<S> + use<S> {
    marked(
        Marks::default().unsaved(palette, unsaved, Dot::new(scale, DotAt::Centre)),
        sized_box(flex_col(())).dims(((DOT * scale).px(), height.px())),
    )
}
