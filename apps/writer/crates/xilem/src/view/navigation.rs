use super::*;
use xilem::masonry::properties::LineBreaking;
use xilem::view::button;
const PAGE: usize = 8;

pub(super) fn navigation(state: &mut Writer) -> impl WidgetView<Writer> + use<> {
    let closing = state.phase == Phase::ConfirmClose;
    const FILE_PAGE: usize = 4;
    let pages = state.project.documents().len().div_ceil(FILE_PAGE).max(1);
    state.file_page = state.file_page.min(pages - 1);
    let files = state
        .project
        .documents()
        .iter()
        .skip(state.file_page * FILE_PAGE)
        .take(FILE_PAGE)
        .map(|document| {
            let path = document.path.clone();
            let open = state
                .project
                .sessions()
                .find(|session| session.path() == path);
            let current = state.project.current().path() == path;
            button(
                label(format!(
                    "{}{}{}",
                    if current { "Current · " } else { "" },
                    document.key.as_str(),
                    if open.is_some_and(|session| session.dirty()) {
                        " · Unsaved"
                    } else {
                        ""
                    }
                ))
                .prop(LineBreaking::WordWrap),
                move |state: &mut Writer| state.switch(&path),
            )
            .disabled(closing)
        })
        .collect::<Vec<_>>();
    let height = match files.len() {
        0 => 0.,
        1 => 64.,
        2 => 100.,
        _ => 140.,
    };
    sized_box(
        flex_col((
            label("Project files"),
            sized_box(
                portal(
                    flex_col(files)
                        .cross_axis_alignment(CrossAxisAlignment::Stretch)
                        .gap(8.px()),
                )
                .constrain_horizontal(true),
            )
            .width(250.px())
            .height(height.px()),
            (pages > 1).then(|| {
                flex_row((
                    text_button("Previous files", |state: &mut Writer| {
                        state.file_page = state.file_page.saturating_sub(1)
                    })
                    .disabled(closing || state.file_page == 0),
                    text_button("Next files", |state: &mut Writer| state.file_page += 1)
                        .disabled(closing || state.file_page + 1 >= pages),
                ))
                .gap(8.px())
            }),
            (pages > 1).then(|| label(format!("Files · page {} of {pages}", state.file_page + 1))),
            outline(state),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(12.px()),
    )
    .width(250.px())
}

fn outline(state: &mut Writer) -> impl WidgetView<Writer> + use<> {
    let closing = state.phase != Phase::Writing;
    let blocks = state.project.current().model().document().block_summaries();
    let pages = blocks.len().div_ceil(PAGE).max(1);
    state.beat_page = state.beat_page.min(pages - 1);
    let entries = blocks
        .iter()
        .skip(state.beat_page * PAGE)
        .take(PAGE)
        .map(|block| {
            let id = block.id().to_string();
            let current = state.reading.selected.as_deref() == Some(&id);
            button(
                label(format!(
                    "{} · {}{}{}",
                    if current { "Current beat" } else { "Beat" },
                    id,
                    if block.is_default() {
                        " · Default"
                    } else {
                        ""
                    },
                    if block.has_end_destination() {
                        " · END"
                    } else {
                        ""
                    }
                ))
                .prop(LineBreaking::WordWrap),
                move |state: &mut Writer| state.inspect(id.clone()),
            )
            .disabled(closing)
        })
        .collect::<Vec<_>>();
    let height = match entries.len() {
        0 => 0.,
        1 => 64.,
        2 => 120.,
        3 => 180.,
        _ => 220.,
    };
    flex_col((
        label("Beats"),
        sized_box(
            portal(
                flex_col(entries)
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .gap(8.px()),
            )
            .constrain_horizontal(true),
        )
        .width(250.px())
        .height(height.px()),
        (pages > 1).then(|| {
            flex_row((
                text_button("Previous beats", |state: &mut Writer| {
                    state.beat_page = state.beat_page.saturating_sub(1)
                })
                .disabled(closing || state.beat_page == 0),
                text_button("Next beats", |state: &mut Writer| state.beat_page += 1)
                    .disabled(closing || state.beat_page + 1 >= pages),
            ))
            .gap(8.px())
        }),
        (pages > 1).then(|| label(format!("Beats · page {} of {pages}", state.beat_page + 1))),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
}
