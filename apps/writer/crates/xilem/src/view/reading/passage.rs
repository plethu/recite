//! A passage occupies the same manuscript row while its single draft is edited.
use super::*;
use crate::presentation::control_style::{ControlStyle, styled};
use crate::presentation::marks::{Mark, marked};
use crate::presentation::space;
use crate::presentation::{Sheet, disclosure_width, print, route_toggle};
use crate::reading::PassageNumber;
use crate::view::workspace::grid::{DESTINATION_GUTTER, GUTTER, ROUTE_PADDING};
use crate::view::workspace::{Grid, Layout};
use recite_core::STABLE_ID_FIELD_PREFIX;
use recite_writer_model::{Passage, PassageKind};
use xilem::masonry::kurbo::Axis;
use xilem::masonry::properties::LineBreaking;
use xilem::view::{FlexSpacer, button};
#[cfg(test)]
mod tests;

/// A route button's padding and border before its text, at `scale`.
pub(in crate::view) fn route_inset(scale: f32) -> f64 {
    ROUTE_PADDING * f64::from(scale) + 1.
}
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
                                (DESTINATION_GUTTER - GUTTER - route_inset(state.ui_scale()))
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

/// Source labels either reserve their own line or share a stacked destination's target height.
enum CaptionPlacement {
    Under { margin: f64 },
    BesideDestination { width: f64 },
}

fn metadata(
    state: &Writer,
    passage: &Passage,
    active: bool,
    placement: CaptionPlacement,
) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let caption_size = Sheet::Number.size(state.reading_size());
    let metadata =
        if active && (state.editing_passage(&passage.id) || state.session.model().has_draft()) {
            format!("{STABLE_ID_FIELD_PREFIX}{}", passage.id)
        } else {
            String::new()
        };
    match placement {
        CaptionPlacement::BesideDestination { width } => {
            let caption_width = width * 0.25;
            Either::A(
                flex_row((
                    route(state, passage, (width - caption_width - 8.).max(80.)),
                    print::id(scale, metadata)
                        .text_size(caption_size)
                        .prop(LineBreaking::Clip)
                        .color(state.palette.muted)
                        .width(caption_width.px()),
                ))
                .cross_axis_alignment(CrossAxisAlignment::Center)
                .gap(space::SIBLINGS.px())
                .width(width.px()),
            )
        }
        CaptionPlacement::Under { margin } => Either::B(
            sized_box(
                flex_row((
                    flex_col(()).dims((margin.px(), 0.px())),
                    print::id(scale, metadata)
                        .text_size(caption_size)
                        .color(state.palette.muted),
                ))
                .gap(space::SIBLINGS.px()),
            )
            // Only clearance below the reply's descenders takes space; the caption itself
            // overhangs into the column gap.
            .padding(Padding::top(CAPTION_CLEARANCE.px()))
            .height(CAPTION_CLEARANCE.px()),
        ),
    }
}

fn route(state: &Writer, passage: &Passage, width: f64) -> Option<impl WidgetView<Writer> + use<>> {
    let PassageKind::Choice {
        destination: Some(destination),
    } = &passage.kind
    else {
        return None;
    };
    if destination == "END" {
        return None;
    }
    let target = destination.clone();
    let size = Sheet::Destination.size(state.reading_size());
    Some(
        sized_box(styled(
            ControlStyle::Action,
            button(
                route_toggle(
                    size / 14.,
                    state.destination_expanded(destination),
                    state.palette.teal_text,
                    size,
                    // Sized with the manuscript it belongs to, not with the interface chrome.
                    Sheet::Destination
                        .label(state.reading_size(), display_name(destination))
                        .prop(LineBreaking::WordWrap)
                        // Fill the content box so the text starts at the column's left edge.
                        .width(
                            (width
                                - 2. * (12. * f64::from(state.ui_scale()) + 1.)
                                - disclosure_width(size / 14.))
                            .max(24.)
                            .px(),
                        )
                        .color(state.palette.teal_text),
                ),
                move |state: &mut Writer| state.toggle_destination_preview(target.clone()),
            )
            .disabled(
                state.phase() != Phase::Writing || state.work.busy() || state.navigation.busy(),
            )
            .background_color(state.palette.surface),
        ))
        .width(width.px()),
    )
}
