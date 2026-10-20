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
            .height(140.px()),
            flex_row((
                text_button("Previous files", |state: &mut Writer| {
                    state.file_page = state.file_page.saturating_sub(1)
                })
                .disabled(closing || state.file_page == 0),
                text_button("Next files", |state: &mut Writer| state.file_page += 1)
                    .disabled(closing || state.file_page + 1 >= pages),
            ))
            .gap(8.px()),
            label(format!("Files · page {} of {pages}", state.file_page + 1)),
            outline(state),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(12.px()),
    )
    .width(250.px())
}

fn outline(state: &mut Writer) -> impl WidgetView<Writer> + use<> {
    let closing = state.phase == Phase::ConfirmClose;
    let model = state.project.current().model();
    let passages = model.document().passage_snapshot().unwrap_or_default();
    let pages = passages.len().div_ceil(PAGE).max(1);
    state.page = state.page.min(pages - 1);
    let entries = passages
        .iter()
        .skip(state.page * PAGE)
        .take(PAGE)
        .map(|passage| {
            let id = passage.id.clone();
            let current = model.view() == &View::Passage(id.clone());
            let excerpt: String = passage.text.chars().take(12).collect();
            text_button(
                format!("{}{}…", if current { "Current · " } else { "" }, excerpt),
                move |state: &mut Writer| state.select(id.clone()),
            )
            .disabled(closing)
        })
        .collect::<Vec<_>>();
    sized_box(
        flex_col((
            label("Passages"),
            flex_col(entries)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .gap(8.px()),
            flex_row((
                text_button("Previous", |state: &mut Writer| {
                    state.page = state.page.saturating_sub(1)
                })
                .disabled(closing || state.page == 0),
                text_button("Next", |state: &mut Writer| state.page += 1)
                    .disabled(closing || state.page + 1 >= pages),
            ))
            .gap(8.px()),
            label(format!("Page {} of {pages}", state.page + 1)),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(12.px()),
    )
    .width(250.px())
}
