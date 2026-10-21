use super::*;
use crate::attention::{Pane, Region};
use crate::presentation::control_style::{ControlStyle, styled};
use crate::presentation::{group::named, print};
use crate::{app::picker::Search, map::Workspace};
use recite_writer_session::choices::display_name;
use xilem::masonry::properties::LineBreaking;
use xilem::view::{button, portal};
const PAGE: usize = 8;
const UNPAGED: usize = 12;

pub(super) fn navigation(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let documents = if state.session.project().is_some() {
        Either::A(project_files(state))
    } else {
        Either::B(examples(state))
    };
    sized_box(named(
        "Scene navigation",
        portal(
            flex_col((
                (state.visible_workspace() != Workspace::Map).then(|| search_controls(state)),
                documents,
                outline(state),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .gap(8.px())
            .padding(12.px()),
        )
        .constrain_horizontal(true),
    ))
    .width(state.drawer_width().px())
    .background_color(state.palette.canvas)
    .prop(Region(Some(Pane::Navigation)))
}
fn search_controls(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let blocked = state.phase != Phase::Writing || state.navigation.busy() || state.work.busy();
    flex_col((
        button(
            label(state.ui_scale(), "Search project words…")
                .prop(LineBreaking::WordWrap)
                .width((state.drawer_width() - 48.).px()),
            |state: &mut Writer| state.pending_search = Some(Search::ProjectText),
        )
        .disabled(blocked || state.session.project().is_none()),
        button(
            label(state.ui_scale(), "Find scene or beat…")
                .prop(LineBreaking::WordWrap)
                .width((state.drawer_width() - 48.).px()),
            |state: &mut Writer| state.pending_search = Some(Search::GoTo),
        )
        .disabled(blocked),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Stretch)
    .gap(4.px())
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
            styled(
                ControlStyle::Navigation,
                button(
                    label(
                        scale,
                        format!(
                            "{}{} · {}{}",
                            if current { "▾ " } else { "▸ " },
                            display_name(
                                document
                                    .key
                                    .as_str()
                                    .rsplit('/')
                                    .next()
                                    .unwrap_or(document.key.as_str())
                            ),
                            document.key,
                            if dirty { " · Unsaved" } else { "" }
                        ),
                    )
                    .color(if current {
                        state.palette.accent
                    } else {
                        state.palette.ink
                    })
                    .prop(LineBreaking::WordWrap)
                    .width((state.drawer_width() - 48.).px()),
                    move |state: &mut Writer| state.switch(&path),
                )
                .disabled(closing)
                .background_color(state.palette.canvas),
            )
        })
        .collect::<Vec<_>>();
    flex_col((
        print::caps(scale, "SCENES").color(state.palette.muted),
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
    .gap(12.px())
    .width((state.drawer_width() - 24.).px())
}

fn outline(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let closing = state.phase != Phase::Writing;
    let blocks = state.session.model().document().block_summaries();
    // Short scenes use the slots otherwise occupied by pagination controls.
    let page_size = if blocks.len() <= UNPAGED {
        UNPAGED
    } else {
        PAGE
    };
    let pages = blocks.len().div_ceil(page_size).max(1);
    let page = state.beat_page.min(pages - 1);
    let entries = blocks
        .iter()
        .skip(page * page_size)
        .take(page_size)
        .map(|block| {
            let id = block.id().to_string();
            let current = state.reading.selected.as_deref() == Some(&id);
            styled(
                ControlStyle::Navigation,
                button(
                    label(
                        scale,
                        format!(
                            "{}{}{}{}",
                            if current { "◆ " } else { "" },
                            if block.is_default() { "Start · " } else { "" },
                            display_name(&id),
                            if block.has_end_destination() {
                                " · end"
                            } else {
                                ""
                            }
                        ),
                    )
                    .prop(LineBreaking::WordWrap)
                    .width((state.drawer_width() - 48.).px()),
                    move |state: &mut Writer| state.inspect(id.clone()),
                )
                .disabled(closing)
                .background_color(if current {
                    state.palette.plate
                } else {
                    state.palette.canvas
                }),
            )
        })
        .collect::<Vec<_>>();
    flex_col((
        flex_col(entries)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .gap(2.px()),
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
                    styled(
                        ControlStyle::Navigation,
                        button(
                            label(
                                scale,
                                format!(
                                    "{}{}",
                                    if example.name
                                        == state.session.model().document().key().as_str()
                                    {
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
                        .disabled(state.phase != Phase::Writing),
                    )
                })
                .collect::<Vec<_>>(),
        )
        .gap(8.px()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
}
