use super::*;
use xilem::masonry::properties::LineBreaking;
use xilem::view::button;
const PAGE: usize = 8;

pub(super) fn navigation(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let documents = if state.session.project().is_some() {
        Either::A(project_files(state))
    } else {
        Either::B(examples(state))
    };
    sized_box(
        flex_col((documents, outline(state)))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(12.px()),
    )
    .width(state.navigation_width().px())
}
fn project_files(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let closing = state.phase != Phase::Writing;
    const FILE_PAGE: usize = 4;
    let pages = state
        .session
        .project()
        .map_or(&[][..], |project| project.documents())
        .len()
        .div_ceil(FILE_PAGE)
        .max(1);
    let page = state.file_page.min(pages - 1);
    let files = state
        .session
        .project()
        .map_or(&[][..], |project| project.documents())
        .iter()
        .skip(page * FILE_PAGE)
        .take(FILE_PAGE)
        .map(|document| {
            let path = document.path.clone();
            let project = state.session.project();
            let dirty = project.is_some_and(|project| {
                project
                    .sessions()
                    .any(|session| session.path() == path && session.dirty())
            });
            let current = project.is_some_and(|project| project.current().path() == path);
            button(
                label(
                    scale,
                    format!(
                        "{}{}{}",
                        if current { "Current · " } else { "" },
                        document.key.as_str(),
                        if dirty { " · Unsaved" } else { "" }
                    ),
                )
                .prop(LineBreaking::WordWrap),
                move |state: &mut Writer| state.switch(&path),
            )
            .disabled(closing)
        })
        .collect::<Vec<_>>();
    sized_box(
        flex_col((
            label(scale, "Project files"),
            flex_col(files)
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .gap(8.px()),
            (pages > 1).then(|| {
                flex_col((
                    text_button(scale, "Previous files", move |state: &mut Writer| {
                        state.file_page = page.saturating_sub(1)
                    })
                    .disabled(closing || page == 0),
                    text_button(scale, "Next files", move |state: &mut Writer| {
                        state.file_page = (page + 1).min(pages - 1)
                    })
                    .disabled(closing || page + 1 >= pages),
                ))
                .gap(8.px())
            }),
            (pages > 1).then(|| label(scale, format!("Files · page {} of {pages}", page + 1))),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(12.px()),
    )
    .width(state.navigation_width().px())
}

fn outline(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let closing = state.phase != Phase::Writing;
    let blocks = state.session.model().document().block_summaries();
    let pages = blocks.len().div_ceil(PAGE).max(1);
    let page = state.beat_page.min(pages - 1);
    let entries = blocks
        .iter()
        .skip(page * PAGE)
        .take(PAGE)
        .map(|block| {
            let id = block.id().to_string();
            let current = state.reading.selected.as_deref() == Some(&id);
            button(
                label(
                    scale,
                    format!(
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
                    ),
                )
                .prop(LineBreaking::WordWrap),
                move |state: &mut Writer| state.inspect(id.clone()),
            )
            .disabled(closing)
        })
        .collect::<Vec<_>>();
    flex_col((
        label(scale, "Beats"),
        flex_col(entries)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .gap(8.px()),
        (pages > 1).then(|| {
            flex_col((
                text_button(scale, "Previous beats", move |state: &mut Writer| {
                    state.beat_page = page.saturating_sub(1)
                })
                .disabled(closing || page == 0),
                text_button(scale, "Next beats", move |state: &mut Writer| {
                    state.beat_page = (page + 1).min(pages - 1)
                })
                .disabled(closing || page + 1 >= pages),
            ))
            .gap(8.px())
        }),
        (pages > 1).then(|| label(scale, format!("Beats · page {} of {pages}", page + 1))),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
}

fn examples(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    flex_col((
        label(scale, "Temporary examples"),
        flex_col(
            recite_writer_model::WRITER_EXAMPLES
                .iter()
                .enumerate()
                .map(|(index, example)| {
                    button(
                        label(
                            scale,
                            format!(
                                "{}{}",
                                if example.name == state.session.model().document().key().as_str() {
                                    "Current · "
                                } else {
                                    ""
                                },
                                example.name
                            ),
                        )
                        .prop(LineBreaking::WordWrap),
                        move |state: &mut Writer| state.select_example(index),
                    )
                    .disabled(state.phase != Phase::Writing)
                })
                .collect::<Vec<_>>(),
        )
        .gap(8.px()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
}
