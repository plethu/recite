//! A passage occupies the same manuscript row while its single draft is edited.
use super::*;
use crate::presentation::control_inset;
use crate::presentation::control_style::{ControlStyle, styled};
use crate::presentation::grid::Grid;
use crate::presentation::grid::{DESTINATION_GUTTER, GUTTER};
use crate::presentation::marks::{Mark, marked};
use crate::presentation::space;
use crate::presentation::{Sheet, disclosure_width, print, route_toggle};
use crate::reading::PassageNumber;
use crate::view::workspace::Layout;
use recite_core::STABLE_ID_FIELD_PREFIX;
use recite_writer_model::{Passage, PassageKind};
use xilem::masonry::kurbo::Axis;
use xilem::masonry::properties::LineBreaking;
use xilem::view::{FlexSpacer, button};
mod metadata;
#[cfg(test)]
mod tests;
pub(super) use metadata::route_to;
use metadata::{CaptionPlacement, metadata, route};

/// Space between a reply and its overhanging source label.
const CAPTION_CLEARANCE: f64 = 4.;

/// The grid every manuscript row is set on.
pub(in crate::view) fn grid(state: &Writer) -> Grid {
    Layout::resolve(state).grid
}

/// Where dialogue text starts in a row, for actions that line up with it.
pub(in crate::view) fn text_column(state: &Writer) -> f64 {
    grid(state).text()
}

/// What a row's margin column holds: a speaker, or a reply's number with "REPLIES" beside the
/// first. Labels sit right-aligned against the gutter; marks sit at the column's left edge.
pub(in crate::view) fn margin_label(
    passage: &Passage,
    reply: Option<usize>,
) -> (String, Option<usize>) {
    match &passage.kind {
        PassageKind::Dialogue { speaker } => {
            (speaker.as_deref().unwrap_or("").to_uppercase(), None)
        }
        PassageKind::Choice { .. } => (
            if reply == Some(1) {
                "REPLIES".into()
            } else {
                String::new()
            },
            reply,
        ),
    }
}

/// The margin column's content for a row: right-aligned and baseline-aligned to the first line.
pub(in crate::view) fn margin<S: 'static>(
    state: &Writer,
    grid: &Grid,
    label: String,
    number: Option<usize>,
) -> impl WidgetView<S> + use<S> {
    let reading = state.reading_size();
    let width = if grid.margin > 0. {
        grid.margin
    } else {
        grid.measure
    };
    flex_row((
        FlexSpacer::Flex(if grid.margin > 0. { 1. } else { 0. }),
        (!label.is_empty()).then(|| {
            Sheet::Label
                .label(reading, label)
                .color(state.palette.accent)
                .prop(LineBreaking::Clip)
        }),
        number.map(|number| {
            Sheet::Number
                .label(reading, number.to_string())
                .color(state.palette.muted)
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::FirstBaseline)
    .gap(space::UNIT.px())
    .width(width.px())
}

/// The mark a passage's row carries: unsaved while it holds a draft, current while selected.
pub(in crate::view) fn passage_mark(state: &Writer, active: bool) -> Option<Mark> {
    active.then(|| {
        if state.session.model().has_draft() {
            Mark::Unsaved
        } else {
            Mark::Current
        }
    })
}

pub(super) fn row(
    state: &Writer,
    number: PassageNumber,
    passage: &Passage,
    reply: Option<usize>,
    width: f64,
) -> impl WidgetView<Writer> + use<> {
    let reading = state.reading_size();
    let grid = grid(state);
    let wide = grid.margin > 0.;
    // Nested groups indent the row; the margin column and the gutters keep their widths.
    let destination = if reply.is_some() {
        grid.destination
    } else {
        0.
    };
    let column = destination > 0.;
    let measure = (width
        - grid.text()
        - if grid.destination > 0. {
            DESTINATION_GUTTER + grid.destination
        } else {
            0.
        })
    .max(120.);
    let (label, numbered) = margin_label(passage, reply);
    // Replies take the 0.9R tier once fields can set their own leading; until then a smaller
    // reply would shrink its field's accessible line box below the reference.
    let size = Sheet::Dialogue.size(reading);
    let active = matches!(state.session.model().view(), View::Passage(id) if id == &passage.id);
    let stacked_reply = !column
        && matches!(&passage.kind, PassageKind::Choice { destination: Some(target) } if target != "END");
    flex_col((
        flex_col((
            marked(
                &state.palette,
                passage_mark(state, active),
                flex_row((
                    (wide || !label.is_empty() || numbered.is_some())
                        .then(|| margin(state, &grid, label, numbered)),
                    super::super::manuscript::passage_editor(state, passage, number, measure, size),
                    column
                        .then(|| route(state, passage, destination))
                        .flatten()
                        .map(|route| {
                            // The route button's own padding takes the rest of the gutter.
                            sized_box(route).padding(Padding::left(
                                (DESTINATION_GUTTER - GUTTER - control_inset(state.ui_scale()))
                                    .max(0.)
                                    .px(),
                            ))
                        }),
                ))
                .direction(if wide {
                    Axis::Horizontal
                } else {
                    Axis::Vertical
                })
                .cross_axis_alignment(if wide {
                    CrossAxisAlignment::FirstBaseline
                } else {
                    CrossAxisAlignment::Start
                })
                .gap(if wide { GUTTER } else { space::UNIT }.px()),
            ),
            // The source label paints into the gap below its passage without taking space, so
            // selecting or typing never moves a row and resting rows keep their rhythm.
            (!stacked_reply).then(|| {
                metadata(
                    state,
                    passage,
                    active,
                    CaptionPlacement::Under {
                        margin: grid.text(),
                    },
                )
            }),
        ))
        .gap(0.px()),
        stacked_reply.then(|| {
            sized_box(metadata(
                state,
                passage,
                active,
                CaptionPlacement::BesideDestination {
                    width: width - grid.text(),
                },
            ))
            .padding(Padding::left(grid.text().px()))
        }),
        active
            .then(|| super::refusal::panel(state, measure))
            .flatten()
            .map(|notice| sized_box(notice).padding(Padding::left(grid.text().px()))),
        match &passage.kind {
            PassageKind::Choice {
                destination: Some(target),
            } => super::super::reference::destination(state, target, width),
            _ => None,
        },
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::UNIT.px())
    // Replies open as a group 24px below the dialogue; each further reply keeps the 12px rhythm.
    .padding(Padding::top(
        if reply == Some(1) { space::TEXT } else { 0. }.px(),
    ))
}

/// How a destination is set: as a route button, whose own padding brings its words in, or as a
/// read-only label padded by the same inset so both read at one place.
#[derive(Clone, Copy, PartialEq)]
pub(super) enum RouteFace {
    Button,
    Label,
}
impl RouteFace {
    fn inset(self, scale: f32) -> f64 {
        match self {
            Self::Button => 0.,
            Self::Label => control_inset(scale),
        }
    }
}

/// A beat's continuation on the grid: `margin` in the margin column, and `route` in the
/// destination column where there is one, or under the text column where destinations wrap.
pub(super) fn continuation_row<M: WidgetView<Writer>, R: WidgetView<Writer>>(
    state: &Writer,
    grid: &Grid,
    margin: Option<M>,
    route: R,
    face: RouteFace,
) -> impl WidgetView<Writer> + use<M, R> {
    let scale = state.ui_scale();
    let column = grid.destination > 0.;
    let pad = if column {
        // Like a reply's route: the button's padding takes the rest of the gutter.
        (DESTINATION_GUTTER - GUTTER - control_inset(scale)).max(0.) + face.inset(scale)
    } else {
        face.inset(scale)
    };
    flex_row((
        flex_row((FlexSpacer::Flex(1.), margin)).width(grid.margin.px()),
        column.then(|| flex_col(()).width(grid.measure.px())),
        sized_box(route).padding(Padding::left(pad.px())),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .gap(grid.gutter().px())
}
