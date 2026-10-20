use super::*;
use crate::reading::{Content, Row};
use recite_writer_model::{Passage, PassageKind, RouteOwner};
use xilem::{AnyWidgetView, style::Padding};
const PAGE: usize = 16;

pub(super) fn repair(state: &Writer, error: &str) -> impl WidgetView<Writer> + use<> {
    flex_col((
        label(state.ui_scale(), "Source editing in your editor"),
        paragraph(state.ui_scale(), "Choose Open in editor to repair the source, then Reload (keep recovery copy). Your source bytes are preserved.", state.script_width()),
        paragraph(state.ui_scale(), error.to_owned(), state.script_width()),
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
    let entries = rows
        .iter()
        .enumerate()
        .skip(page * PAGE)
        .take(PAGE)
        .map(|(index, entry)| row(state, id, index, entry))
        .collect::<Vec<_>>();
    flex_col((
        label(scale, format!("Beat · {id}")).text_size(22. * scale),
        flex_col(entries)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(20.px()),
        rows.is_empty().then(|| {
            prose(
                scale,
                "No dialogue yet. Add a line or reply, or edit the source in your editor.",
            )
        }),
        pagination(state, page, pages),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(16.px())
}
fn row(state: &Writer, beat: &str, index: usize, row: &Row) -> impl WidgetView<Writer> + use<> {
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
        content(state, beat, index, row, width),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(6.px())
    .padding(Padding::left(inset.px()))
}
fn content(
    state: &Writer,
    beat: &str,
    index: usize,
    row: &Row,
    width: f64,
) -> Box<AnyWidgetView<Writer>> {
    match &row.content {
        Content::Passage(item) => passage(state, index, item, width).boxed(),
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
fn passage(
    state: &Writer,
    index: usize,
    passage: &Passage,
    width: f64,
) -> impl WidgetView<Writer> + use<> {
    let id = passage.id.clone();
    let caption = match &passage.kind {
        PassageKind::Dialogue { speaker } => speaker.clone().unwrap_or_else(|| "Narration".into()),
        PassageKind::Choice { destination } => format!(
            "Reply → {}",
            destination.as_deref().unwrap_or("Choose a destination")
        ),
    };
    flex_col((
        label(state.ui_scale(), caption),
        flex_row((sized_box(crate::presentation::manuscript_text(
            state.reading_size(),
            passage.text.clone(),
        ))
        .width(width.px()),)),
        text_button(
            state.ui_scale(),
            format!("Edit passage {}", index + 1),
            move |state: &mut Writer| state.select(id.clone()),
        )
        .disabled(state.phase != Phase::Writing),
        matches!(passage.kind, PassageKind::Choice { .. })
            .then(|| reply_actions(state, &passage.id)),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
}
fn reply_actions(state: &Writer, id: &str) -> impl WidgetView<Writer> + use<> {
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
    flex_col((
        paragraph(state.ui_scale(), format!("Continue → {target}"), width),
        beat.map(|id| {
            let owner = RouteOwner::Beat(id.to_owned());
            text_button(
                state.ui_scale(),
                "Change continuation…",
                move |state: &mut Writer| state.choose_destination(owner.clone()),
            )
            .disabled(state.phase != Phase::Writing || state.navigation.busy() || state.work.busy())
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
