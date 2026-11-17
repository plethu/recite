//! A passage occupies the same manuscript row while its single draft is edited.
use super::*;
use crate::presentation::control_style::{ControlStyle, styled};
use crate::presentation::grid::Grid;
use crate::presentation::grid::{DESTINATION_GUTTER, GUTTER};
use crate::presentation::marks::{Mark, marked};
use crate::presentation::space;
use crate::presentation::{
    Page, Sheet, disclosure_width, overhang, print, route_name, route_toggle,
};
use crate::presentation::{control_inset, label_inset};
use crate::reading::PassageNumber;
use crate::view::workspace::Layout;
use recite_core::STABLE_ID_FIELD_PREFIX;
use recite_writer_model::{Passage, PassageKind};
use xilem::masonry::kurbo::Axis;
use xilem::masonry::properties::LineBreaking;
use xilem::view::{FlexSpacer, button};
mod choice;
mod continuation;
mod metadata;
#[cfg(test)]
mod tests;
pub(super) use continuation::{RouteFace, continuation_row};
pub(super) use metadata::route_to;
use metadata::{CaptionPlacement, metadata, route};

/// Space between a reply and its overhanging source label.
const CAPTION_CLEARANCE: f64 = 4.;
/// The spacing between reply rows: the caption's clearance and the text rhythm.
const ROUTE_REACH: f64 = CAPTION_CLEARANCE + space::TEXT;

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
    let page = Page::of(state);
    let width = if grid.margin > 0. {
        grid.margin
    } else {
        grid.measure
    };
    // An empty margin still sets a baseline, at its top, so it never lowers a speakerless line.
    let empty = label.is_empty() && number.is_none();
    flex_row((
        FlexSpacer::Flex(if grid.margin > 0. { 1. } else { 0. }),
        empty.then(|| sized_box(flex_col(())).dims((0.px(), 0.px()))),
        (!label.is_empty()).then(|| {
            Sheet::Label
                .label(page, label)
                .color(state.palette.accent)
                .prop(LineBreaking::Clip)
        }),
        number.map(|number| {
            Sheet::Number
                .label(page, number.to_string())
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
    let page = Page::of(state);
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
    let size = Sheet::text(reply).size(page);
    let active = matches!(state.session.model().view(), View::Passage(id) if id == &passage.id);
    let speaking = active && matches!(passage.kind, PassageKind::Dialogue { .. });
    // A written reply always shows its destination control; a resting one only a route.
    let stacked_reply = !column
        && match &passage.kind {
            PassageKind::Choice { destination } => {
                active || destination.as_deref().is_some_and(|target| target != "END")
            }
            PassageKind::Dialogue { .. } => false,
        };
    flex_col((
        flex_col((
            marked(
                &state.palette,
                passage_mark(state, active),
                flex_row((
                    // Options, not one Either, so each keeps its own baselines in the row.
                    speaking.then(|| choice::speaker(state, &grid, passage)),
                    (!speaking && (wide || !label.is_empty() || numbered.is_some()))
                        .then(|| margin(state, &grid, label, numbered)),
                    super::super::manuscript::passage_editor(state, passage, number, measure, size),
                    column
                        .then(|| route(state, passage, active, destination))
                        .flatten()
                        .map(|route| hang_route(state, route)),
                    column
                        .then(|| metadata::choice(state, passage, active, destination))
                        .flatten()
                        .map(|choice| hang_route(state, choice)),
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
                .gap(
                    if wide && speaking {
                        GUTTER - label_inset()
                    } else if wide {
                        GUTTER
                    } else {
                        space::UNIT
                    }
                    .px(),
                ),
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

/// A route in the destination column: the button's own padding takes the rest of the gutter.
/// Its target reaches into the spacing between reply rows, so a target taller than the reply's
/// line sets the row no taller than the line; neighbouring targets meet but never overlap.
fn hang_route<V: WidgetView<Writer>>(state: &Writer, route: V) -> impl WidgetView<Writer> + use<V> {
    overhang(
        sized_box(route).padding(Padding::left(
            (DESTINATION_GUTTER - GUTTER - control_inset(state.ui_scale()))
                .max(0.)
                .px(),
        )),
        ROUTE_REACH,
    )
}
