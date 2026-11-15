use super::*;
use crate::presentation::pane;
mod outline;
use crate::app::picker::Search;
use crate::attention::{Pane, Region};
use crate::map::Workspace;
use crate::navigation::rail::{FILE_PAGE, beat_page_size};
use crate::presentation::control_style::{ControlStyle, styled};
use crate::presentation::marks::{Mark, marked};
use crate::presentation::row::{listed, named_row, row};
use crate::presentation::sheet_text::one_line;
use crate::presentation::space;
use crate::presentation::tokens::sidebar;
use crate::presentation::{Icon, SMALL, Type, control_inset, mark};
use crate::presentation::{disclosure, disclosure_width, group::named};
use outline::outline;
use recite_writer_session::choices::display_name;
use xilem::masonry::parley::style::FontWeight;
use xilem::masonry::properties::LineBreaking;
use xilem::view::button;

/// The scene list's width: as allocated while shown, and its preferred width while retained
/// hidden, so its rows keep their layout.
pub(crate) fn sidebar_width(state: &Writer) -> f64 {
    let width = state.navigation_width();
    if width > 0. {
        width
    } else {
        state.drawer_width()
    }
}

pub(super) fn navigation(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let documents = if state.session.project_root().is_some() {
        Either::A(project_files(state))
    } else {
        Either::B(
            flex_col((examples(state), outline(state)))
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .gap(space::SIBLINGS.px()),
        )
    };
    sized_box(named(
        "Scene navigation",
        pane(
            "Scenes",
            flex_col((search_entry(state), documents))
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .gap(space::KINDS.px())
                .padding(space::KINDS.px()),
        )
        .constrain_horizontal(true),
    ))
    .width(sidebar_width(state).px())
    .prop(Region(Some(Pane::Navigation)))
}

/// One field-shaped entry that opens the search for scenes, beats and words.
fn search_entry(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    styled(
        ControlStyle::Entry,
        named_row(
            SEARCH,
            flex_row((
                mark(scale * SMALL, "", Icon::Search, state.palette.muted),
                one_line(SEARCH, Type::Body, scale, state.palette.muted).flex(1.),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .gap(space::SIBLINGS.px())
            .width((sidebar_width(state) - 2. * space::KINDS - 2. * control_inset(scale)).px()),
            |state: &mut Writer| state.request_search(Search::GoTo),
        )
        .disabled(state.project_action_blocked()),
    )
}
/// The search entry's words and name.
const SEARCH: &str = "Find scene, beat or text…";

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
            let ink = if current {
                state.palette.accent
            } else {
                state.palette.ink
            };
            let caption = if ambiguous {
                format!("{title} · {key}")
            } else {
                title
            };
            // A scene row in Control semibold, its beats nested below it when it is open. Unsaved
            // work carries the halftone mark beside the words that say so.
            let row = marked(
                &state.palette,
                dirty.then_some(Mark::Unsaved),
                styled(
                    ControlStyle::Navigation,
                    button(
                        disclosure(
                            scale,
                            current,
                            ink,
                            one_line(
                                format!("{caption}{}", if dirty { " · Unsaved" } else { "" }),
                                Type::Control,
                                scale,
                                ink,
                            )
                            .weight(FontWeight::SEMI_BOLD)
                            .width(
                                (sidebar_width(state) - sidebar::ROW - disclosure_width(scale))
                                    .px(),
                            ),
                        ),
                        move |state: &mut Writer| state.switch(&path),
                    )
                    .expanded(current)
                    .disabled(closing),
                ),
            );
            flex_col((row, current.then(|| outline(state))))
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .gap(0.px())
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
            .width((sidebar_width(state) - sidebar::ROW).px())
        }),
        state
            .session
            .checked_out()
            .then(|| label(scale, "Project operation in progress")),
        flex_col(files)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .gap(space::SIBLINGS.px()),
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
            .gap(space::SIBLINGS.px())
        }),
        (pages > 1).then(|| label(scale, format!("Files · page {} of {pages}", page + 1))),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::TEXT.px())
    .width((sidebar_width(state) - sidebar::COLUMN).px())
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
        .gap(space::SIBLINGS.px()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::TEXT.px())
}
