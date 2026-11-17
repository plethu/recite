use super::surface::Surface;
use super::*;
use crate::WriterLayout;
use crate::app::picker::Search;
use crate::keyed::column;
use crate::presentation::Icon;
use crate::presentation::control_style::{ControlStyle, styled};
use crate::presentation::grid::Grid;
use crate::presentation::material;
use crate::presentation::print;
use crate::presentation::space;
use crate::presentation::{Page, Sheet, Type};
use crate::reading::{Content, Row};
use recite_core::DocumentKey;
use recite_writer_model::{AnalysisStamp, RouteOwner};
use recite_writer_session::choices::display_name;
mod controls;
mod galley;
pub(crate) use galley::SelectedBeat;
mod passage;
pub(super) use passage::{margin, text_column};
mod refusal;
pub(super) mod viewport;
use xilem::{AnyWidgetView, masonry::kurbo::Axis, style::Padding};

#[derive(Clone, PartialEq)]
enum RowIdentity {
    Passage(DocumentKey, String),
    Structural(AnalysisStamp, usize),
}

pub(super) fn empty(state: &Writer) -> impl WidgetView<Writer> + use<> {
    flex_col((
        // Halftone marks what is not yet written; it sits beside the heading, never behind it.
        flex_row((
            print::section_heading(state.ui_scale(), "No beats yet"),
            material::halftone(
                96. * f64::from(state.ui_scale()),
                20. * f64::from(state.ui_scale()),
                state.palette.accent,
            ),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Center)
        .gap(space::TEXT.px()),
        prose(
            state.ui_scale(),
            "Add a beat to start writing. Comments and existing source are preserved.",
        ),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::SIBLINGS.px())
}
pub(super) fn block(state: &Writer, id: &str, rows: &[Row]) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let range = state.reading.window.range(rows.len());
    let at_end = range.end == rows.len();
    let entries = range
        // A continuation set on its line's row takes no row of its own.
        .filter(|&index| !passage::absorbed(rows, index))
        .map(|index| {
            let key = match &rows[index].content {
                Content::Passage { passage, .. } => RowIdentity::Passage(
                    state.session.model().document().key().clone(),
                    passage.id.clone(),
                ),
                _ => RowIdentity::Structural(
                    state.session.model().document().analysis_stamp(),
                    index,
                ),
            };
            (
                key,
                row(state, id, &rows[index], passage::joined(rows, index)),
            )
        })
        .collect();
    let _ = id;
    let body = flex_col((
        // 12px between rows, and each row's 4px source-label clearance below it.
        column(entries).gap(space::TEXT.px()),
        // In the text column, clear of the current beat's margin bar.
        rows.is_empty().then(|| {
            sized_box(prose(
                scale,
                "No dialogue yet. Add a line or reply, or edit the source in your editor.",
            ))
            .padding(Padding::left(passage::grid(state).text().px()))
        }),
        // Only a beat that really ends the conversation shows the end rule.
        (at_end && passage::ends(rows))
            .then(|| print::end_rule(scale, &state.palette, state.script_width())),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::KINDS.px())
    .padding(Padding::top(space::KINDS.px()));
    let body = flex_col((
        body,
        retained(
            super::manuscript::actions(state),
            !state.attention.is_focused(),
        ),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::TEXT.px());
    galley::galley(state, body)
}
fn row(
    state: &Writer,
    beat: &str,
    row: &Row,
    joined: Option<&str>,
) -> impl WidgetView<Writer> + use<> {
    let inset = row.groups.len().min(3) as f64 * 24.;
    let width = state.script_width() - inset;
    flex_col((
        (!row.groups.is_empty()).then(|| {
            sized_box(
                prose(
                    state.ui_scale(),
                    format!("Within {}", row.groups.join(" → ")),
                )
                .text_size(14.),
            )
            .width(width.px())
        }),
        content(state, beat, row, (width, joined)),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::SIBLINGS.px())
    .padding(Padding::left(inset.px()))
}
fn content(
    state: &Writer,
    beat: &str,
    row: &Row,
    (width, joined): (f64, Option<&str>),
) -> Box<AnyWidgetView<Writer>> {
    match &row.content {
        Content::Passage { passage, number } => passage::row(
            state,
            *number,
            passage,
            row.reply,
            (width, joined.map(|target| (beat, target))),
        )
        .boxed(),
        Content::Heading(text) => sized_box(
            prose(state.ui_scale(), text.clone())
                .text_size(Sheet::Dialogue.size(Page::of(state)))
                .line_height(Sheet::LEADING),
        )
        .width(width.px())
        .boxed(),
        Content::Note(text) => paragraph(state.ui_scale(), text.clone(), width).boxed(),
        Content::Continuation(target) => {
            continuation(state, target, row.groups.is_empty().then_some(beat), width).boxed()
        }
    }
}
pub(super) fn reply_actions(state: &Writer, id: &str) -> impl WidgetView<Writer> + use<> {
    let rule_id = id.to_owned();
    let route_id = id.to_owned();
    let disabled = state.phase() != Phase::Writing || state.navigation.busy() || state.work.busy();
    flex_col((
        text_button(
            state.ui_scale(),
            "Reply rules…",
            move |state: &mut Writer| {
                let mut location = state.current_location();
                location.screen = recite_writer_session::location::Screen::Rules;
                location.passage = Some(rule_id.clone());
                location.source = false;
                state.navigate(crate::navigation::Target::Location(location));
            },
        )
        .disabled(disabled),
        text_button(
            state.ui_scale(),
            "Change reply destination…",
            move |state: &mut Writer| state.choose_destination(RouteOwner::Reply(route_id.clone())),
        )
        .disabled(disabled),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::SIBLINGS.px())
}
/// A beat's continuation on a row of its own, where it follows no line: the end mark in the text
/// column, or its route in the destination column like a reply's (under the text where
/// destinations wrap). The route is the control that changes it; inside a group it only shows
/// where the group leads. Read-only beats set it in the same place.
fn continuation(
    state: &Writer,
    target: &str,
    beat: Option<&str>,
    width: f64,
) -> impl WidgetView<Writer> + use<> {
    let grid = passage::grid(state);
    if target == "END" {
        return Either::A(
            sized_box(print::end_mark(state.ui_scale(), &state.palette))
                .padding(Padding::left(grid.text().px())),
        );
    }
    let route = match beat {
        Some(id) => Either::A(passage::route_choice(
            state,
            (RouteOwner::Beat(id.to_owned()), "continuation"),
            Some(target),
            route_width(&grid),
        )),
        None => Either::B(passage::route_to(state, target, route_width(&grid))),
    };
    Either::B(
        flex_col((
            passage::continuation_row(
                state,
                &grid,
                None::<xilem::view::Label>,
                route,
                passage::RouteFace::Button,
            ),
            beat.is_none()
                .then(|| super::reference::destination(state, target, width))
                .flatten(),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(space::SIBLINGS.px()),
    )
}
/// A continuation's width: the destination column, or the measure where destinations wrap.
fn route_width(grid: &Grid) -> f64 {
    if grid.destination > 0. {
        grid.destination
    } else {
        grid.measure
    }
}
pub(super) use controls::page_controls;
