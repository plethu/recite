use super::*;
use crate::presentation::pane;
mod outline;
#[cfg(test)]
mod tests;
use crate::app::picker::Search;
use crate::attention::{Pane, Region};
use crate::map::Workspace;
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
    // The search entry stays put above the scrolled list, so the list's thumb never crosses it.
    sized_box(named(
        "Scene navigation",
        flex_col((
            flex_col((search_entry(state),))
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .padding(Padding {
                    bottom: 0.px(),
                    ..Padding::all(space::KINDS.px())
                }),
            pane(
                "Scenes",
                flex_col((documents,))
                    .cross_axis_alignment(CrossAxisAlignment::Stretch)
                    .padding(space::KINDS.px()),
            )
            .constrain_horizontal(true)
            .flex(1.),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .gap(0.px()),
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
    // Every file is a row; the scene list scrolls rather than paging.
    let files = documents
        .iter()
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
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::TEXT.px())
    .width((sidebar_width(state) - sidebar::COLUMN).px())
}

/// The temporary examples as list rows: the open one selected, all of them left-aligned.
fn examples(state: &Writer) -> impl WidgetView<Writer> + use<> {
    let scale = state.ui_scale();
    let current = state.session.model().document().key().as_str();
    let width = sidebar_width(state) - sidebar::ROW;
    flex_col((
        label(scale, "Temporary examples"),
        flex_col(
            recite_writer_model::WRITER_EXAMPLES
                .iter()
                .enumerate()
                .map(|(index, example)| {
                    let selected = example.name == current;
                    listed(
                        &state.palette,
                        selected,
                        row(
                            selected,
                            one_line(example.name, Type::Control, scale, state.palette.ink)
                                .width(width.px()),
                            move |state: &mut Writer| state.select_example(index),
                        )
                        .disabled(state.phase() != Phase::Writing),
                    )
                })
                .collect::<Vec<_>>(),
        )
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .gap(0.px()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Stretch)
    .gap(space::TEXT.px())
}
