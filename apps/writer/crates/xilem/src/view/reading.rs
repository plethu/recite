use super::*;
use crate::keyed::column;
use crate::presentation::print;
use crate::reading::{Content, Row};
use recite_core::DocumentKey;
use recite_writer_model::{AnalysisStamp, RouteOwner};
use recite_writer_session::choices::display_name;
mod header;
mod passage;
use crate::reading::PAGE;
use xilem::{
    AnyWidgetView,
    masonry::{kurbo::Axis, properties::ContentColor},
    style::Padding,
    view::divider,
};

#[derive(Clone, PartialEq)]
enum RowIdentity {
    Passage(DocumentKey, String),
    Structural(AnalysisStamp, usize),
}

pub(super) fn repair(state: &Writer, error: &str) -> impl WidgetView<Writer> + use<> {
    flex_col((
        label(state.ui_scale(), "Source editing in your editor"),
        paragraph(state.ui_scale(), "Choose Open in editor to repair the source, then Reload (keep recovery copy). Your source bytes are preserved.", state.script_width()),
        paragraph(state.ui_scale(), error.to_owned(), state.script_width()),
        state.session.project().map(|_| print::action(state.ui_scale(), "Open in editor", state.phase != Phase::Writing || state.external.busy(), Writer::open_editor)),
    )).cross_axis_alignment(CrossAxisAlignment::Start).gap(8.px())
}
pub(super) fn empty(state: &Writer) -> impl WidgetView<Writer> + use<> {
    flex_col((
        label(state.ui_scale(), "No beats yet").text_size(22. * state.ui_scale()),
        prose(
            state.ui_scale(),
            "Add a beat to start writing. Comments and existing source are preserved.",
        ),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
}
pub(super) fn block(state: &Writer, id: &str, rows: &[Row]) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let pages = rows.len().div_ceil(PAGE).max(1);
    let page = state.reading.page.min(pages - 1);
    let start = page * PAGE;
    let end = (start + PAGE).min(rows.len());
    let entries = (start..end)
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
            (key, row(state, id, &rows[index]))
        })
        .collect();
    flex_col((
        header::heading(state, id),
        column(entries).gap(12.px()),
        rows.is_empty().then(|| {
            prose(
                scale,
                "No dialogue yet. Add a line or reply, or edit the source in your editor.",
            )
        }),
        pagination(state, page, pages),
        (page + 1 == pages
            && !matches!(
                rows.last().map(|row| &row.content),
                Some(Content::Continuation(_))
            ))
        .then(|| {
            divider(
                Axis::Horizontal,
                print::meta(scale, "◆ END ◆").color(state.palette.muted),
            )
            .dash_pattern(&[1.px(), 3.px()])
            .prop(ContentColor::new(state.palette.rule))
            .width(state.script_width().px())
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(16.px())
}
fn row(state: &Writer, beat: &str, row: &Row) -> impl WidgetView<Writer> + use<> {
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
        content(state, beat, row, width),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(6.px())
    .padding(Padding::left(inset.px()))
}
fn content(state: &Writer, beat: &str, row: &Row, width: f64) -> Box<AnyWidgetView<Writer>> {
    match &row.content {
        Content::Passage { passage, number } => {
            passage::row(state, *number, passage, row.reply, width).boxed()
        }
        Content::Heading(text) => {
            sized_box(prose(state.ui_scale(), text.clone()).text_size(state.reading_size()))
                .width(width.px())
                .boxed()
        }
        Content::Note(text) => paragraph(state.ui_scale(), text.clone(), width).boxed(),
        Content::Continuation(target) => {
            continuation(state, target, row.groups.is_empty().then_some(beat), width).boxed()
        }
    }
}
pub(super) fn reply_actions(state: &Writer, id: &str) -> impl WidgetView<Writer> + use<> {
    let rule_id = id.to_owned();
    let route_id = id.to_owned();
    let disabled = state.phase != Phase::Writing || state.navigation.busy() || state.work.busy();
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
    .gap(8.px())
}
fn continuation(
    state: &Writer,
    target: &str,
    beat: Option<&str>,
    width: f64,
) -> impl WidgetView<Writer> + use<> {
    let end = target == "END";
    let target = target.to_owned();
    let toggle = target.clone();
    flex_col((
        end.then(|| print::meta(state.ui_scale(), "◆ END ◆").color(state.palette.muted)),
        (!end).then(|| {
            print::action(
                state.ui_scale(),
                format!(
                    "{} {}",
                    if state.destination_expanded(&target) {
                        "▾"
                    } else {
                        "→"
                    },
                    display_name(&target)
                ),
                state.phase != Phase::Writing,
                move |state: &mut Writer| state.toggle_destination_preview(toggle.clone()),
            )
        }),
        (!end)
            .then(|| super::reference::destination(state, &target, width))
            .flatten(),
        beat.map(|id| {
            let owner = RouteOwner::Beat(id.to_owned());
            print::action(
                state.ui_scale(),
                "Change continuation…",
                state.phase != Phase::Writing || state.navigation.busy() || state.work.busy(),
                move |state: &mut Writer| state.choose_destination(owner.clone()),
            )
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
}
fn pagination(
    state: &Writer,
    page: usize,
    pages: usize,
) -> Option<impl WidgetView<Writer> + use<>> {
    let closing = state.phase != Phase::Writing;
    (pages > 1).then(|| {
        flex_row((
            text_button(
                state.ui_scale(),
                "Previous beat entries",
                move |state: &mut Writer| state.reading.page = page.saturating_sub(1),
            )
            .disabled(closing || page == 0),
            label(state.ui_scale(), format!("{} of {pages}", page + 1)),
            text_button(
                state.ui_scale(),
                "Next beat entries",
                move |state: &mut Writer| state.reading.page = (page + 1).min(pages - 1),
            )
            .disabled(closing || page + 1 >= pages),
        ))
        .gap(8.px())
    })
}
