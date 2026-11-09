use super::*;
use crate::app::picker::Search;
use crate::attention::{Pane, Region};
use crate::map::Workspace;
use crate::navigation::rail::{FILE_PAGE, beat_page_size};
use crate::presentation::control_style::{ControlStyle, styled};
use crate::presentation::{group::named, print};
use recite_writer_session::choices::display_name;
use xilem::masonry::parley::style::FontWeight;
use xilem::masonry::properties::LineBreaking;
use xilem::view::{button, portal};
use xilem::{
    masonry::{
        imaging::Painter,
        kurbo::{Affine, BezPath},
    },
    view::canvas,
};

pub(super) fn navigation(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let documents = if state.session.project_root().is_some() {
        Either::A(project_files(state))
    } else {
        Either::B(examples(state))
    };
    sized_box(named(
        "Scene navigation",
        portal(
            flex_col((
                print::caps(state.ui_scale(), "SCENES").color(state.palette.muted),
                search_controls(state),
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
    let blocked = state.project_action_blocked();
    flex_col((
        styled(
            ControlStyle::Navigation,
            button(
                label(state.ui_scale(), "Search project words…")
                    .weight(FontWeight::MEDIUM)
                    .color(state.palette.accent)
                    .prop(LineBreaking::WordWrap)
                    .width((state.drawer_width() - 48.).px()),
                |state: &mut Writer| state.request_search(Search::ProjectText),
            )
            .disabled(blocked || state.session.project().is_none()),
        ),
        styled(
            ControlStyle::Navigation,
            button(
                label(state.ui_scale(), "Find scene or beat…")
                    .weight(FontWeight::MEDIUM)
                    .color(state.palette.accent)
                    .prop(LineBreaking::WordWrap)
                    .width((state.drawer_width() - 48.).px()),
                |state: &mut Writer| state.request_search(Search::GoTo),
            )
            .disabled(blocked),
        ),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Stretch)
    .gap(4.px())
}
fn project_files(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let closing = state.project_action_blocked();
    let documents = state
        .session
        .project()
        .map_or(&[][..], |project| project.documents());
    let pages = documents.len().div_ceil(FILE_PAGE).max(1);
    let page = state.file_page.min(pages - 1);
    let files = documents
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
            let key = document.key.as_str();
            let basename = key.rsplit('/').next().unwrap_or(key);
            let title = display_name(basename);
            let ambiguous = documents.iter().any(|other| {
                other.key != document.key
                    && other
                        .key
                        .as_str()
                        .rsplit('/')
                        .next()
                        .map(display_name)
                        .as_deref()
                        == Some(title.as_str())
            });
            let caption = if ambiguous {
                format!("{title} · {key}")
            } else {
                title
            };
            styled(
                ControlStyle::Navigation,
                button(
                    label(
                        scale,
                        format!(
                            "{}{}{}",
                            if current { "▾ " } else { "▸ " },
                            caption,
                            if dirty { " · Unsaved" } else { "" }
                        ),
                    )
                    .weight(FontWeight::SEMI_BOLD)
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
        state.session.checked_out().then(|| {
            let key = state.session.model().document().key().to_string();
            label(
                scale,
                format!(
                    "{} · {key}{}",
                    display_name(&key),
                    if state.session.current_dirty() {
                        " · Unsaved"
                    } else {
                        ""
                    }
                ),
            )
            .prop(LineBreaking::WordWrap)
            .width((state.drawer_width() - 48.).px())
        }),
        state
            .session
            .checked_out()
            .then(|| label(scale, "Project operation in progress")),
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
    let closing = state.project_action_blocked();
    let blocks = state.session.model().document().block_summaries();
    // Short scenes use the slots otherwise occupied by pagination controls.
    let page_size = beat_page_size(blocks.len());
    let pages = blocks.len().div_ceil(page_size).max(1);
    let page = state.beat_page.min(pages - 1);
    let selected = if state.visible_workspace() == Workspace::Map {
        state.map_selected()
    } else {
        state.reading.selected.as_deref()
    };
    let entries = blocks
        .iter()
        .skip(page * page_size)
        .take(page_size)
        .map(|block| {
            let id = block.id().to_string();
            let current = selected == Some(id.as_str());
            styled(
                ControlStyle::Navigation,
                button(
                    outline_caption(
                        state,
                        current,
                        label(
                            scale,
                            format!(
                                "{}{}{}",
                                if block.is_default() { "Start · " } else { "" },
                                display_name(&id),
                                if block.has_end_destination() {
                                    " · end"
                                } else {
                                    ""
                                }
                            ),
                        )
                        .weight(if current {
                            FontWeight::SEMI_BOLD
                        } else {
                            FontWeight::NORMAL
                        })
                        .prop(LineBreaking::WordWrap),
                    ),
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

fn outline_caption<V: WidgetView<Writer, Widget: Sized>>(
    state: &Writer,
    current: bool,
    text: V,
) -> impl WidgetView<Writer> + use<V> {
    let scale = f64::from(state.ui_scale());
    let teal = state.palette.teal;
    let marker = 12. * scale + 8.;
    let text = text.width((state.drawer_width() - 48. - if current { marker } else { 0. }).px());
    if !current {
        return Either::B(text.padding(xilem::style::Padding::left(marker.px())));
    }
    Either::A(
        flex_row((
            canvas(move |_: &mut Writer, _, recording, _| {
                let mut diamond = BezPath::new();
                diamond.move_to((6., 1.));
                diamond.line_to((11., 6.));
                diamond.line_to((6., 11.));
                diamond.line_to((1., 6.));
                diamond.close_path();
                Painter::new(recording)
                    .fill(diamond, teal)
                    .transform(Affine::scale(scale))
                    .draw();
            })
            .dims(((12. * scale).px(), (12. * scale).px())),
            text,
        ))
        .gap(8.px()),
    )
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
                        .disabled(state.phase() != Phase::Writing),
                    )
                })
                .collect::<Vec<_>>(),
        )
        .gap(8.px()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
}
