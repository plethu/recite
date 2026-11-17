//! Window identity and workspace commands live outside the manuscript's reading measure.
use crate::presentation::material::{Bar, Edge, bar};
use crate::presentation::segments::{tab, tabs};
use crate::presentation::sheet_text::one_line;
use crate::presentation::tokens::{breakpoint, size};
use crate::presentation::{Type, space};
use crate::presentation::{identity, paragraph, print};
use crate::workspaces::rename::inline::modal as rename_modal;
use crate::{
    Phase, Writer,
    app::{App, picker},
};
use xilem::{
    WidgetView,
    masonry::{
        kurbo::Axis,
        layout::AsUnit,
        parley::style::FontWeight,
        widgets::{FlexBasis, FlexParams},
    },
    style::Style,
    view::{CrossAxisAlignment, FlexExt, flex_col, flex_row, sized_box},
};
pub(super) fn app_bar(app: &App) -> impl WidgetView<App> + use<> {
    let scale = crate::presentation::scale(&app.preferences.config.writer);
    // A picker, Settings or a rename's decision holds the window: the tab row behind it is inert,
    // all but Settings' toggle, Workspace.
    let held = app.picker.is_some() || app.writer.as_ref().is_some_and(rename_modal);
    let searching = held || app.settings_open();
    let deciding = app
        .writer
        .as_ref()
        .is_some_and(|writer| writer.phase() != Phase::Writing);
    let blocked = searching || deciding;
    // The tab row stays at every window size and interface scale.
    sized_box(
        flex_col((
            identity_bar(app, scale, (blocked, held || deciding)),
            app.workspace_menu()
                .then(|| workspace_actions(app, scale, searching)),
            (!app.status.is_empty() && (!app.show_welcome() || app.settings_open()))
                .then(|| paragraph(scale, app.status.clone(), 800.)),
            app.pane_hint.then(|| {
                paragraph(
                    scale,
                    "Pane navigation · h left · j down · k up · l right · Esc cancel",
                    900.,
                )
            }),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(space::SIBLINGS.px()),
    )
    // The tab row is one control tall.
    .padding(xilem::style::Padding::from_vh(0.px(), space::KINDS.px()))
}

/// The tab row's identity and commands, inert while `blocked`; Workspace waits only while `held`.
fn identity_bar(
    app: &App,
    scale: f32,
    (blocked, held): (bool, bool),
) -> impl WidgetView<App> + use<> {
    // Writing, focus mode and the tool screens share the toolbar's Workspace menu, so their tab
    // row keeps only the file, the modes and Commands; Settings is a sheet over it.
    let writing = !app.show_welcome() && app.writer.is_some();
    // A compact row with the scene list hidden gives the wordmark's column to the file tab, so
    // the tab keeps its 96px and the row its commands; the window title still names Recite.
    let compact = app.viewport.width < breakpoint::TOOLBAR * f64::from(scale);
    let column = app.writer.as_ref().map_or(220., |writer| {
        writer
            .navigation_width()
            .max(size::TAB_MIN * f64::from(scale))
    });
    let listed = app
        .writer
        .as_ref()
        .is_none_or(|writer| writer.navigation_width() > 0.);
    flex_row((
        (!compact || listed || app.show_welcome()).then(|| {
            sized_box(flex_row((
                // The welcome hero carries the wordmark while it shows.
                (!app.show_welcome()).then(|| identity::wordmark(&app.theme, scale)),
                xilem::view::FlexSpacer::Flex(1.),
            )))
            // The wordmark heads the scene list, or keeps a minimal column while it is hidden.
            .width(column.px())
        }),
        // The file tab takes the row's spare width from a 96px minimum and ends in an ellipsis
        // when that is short of its name; assistive technology reads the whole name.
        app.writer.as_ref().map(|writer| {
            flex_col((bar(
                Some(Bar::solid(app.theme.palette.teal, Edge::Bottom, 0., 0.)),
                sized_box(
                    one_line(
                        writer.session.model().document().key().to_string(),
                        Type::Control,
                        scale,
                        app.theme.palette.ink,
                    )
                    .weight(FontWeight::SEMI_BOLD),
                )
                .padding(xilem::style::Padding::from_vh(
                    space::UNIT.px(),
                    space::SIBLINGS.px(),
                )),
            ),))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            // The tab keeps 96px before the row shares its spare width.
            .flex(FlexParams::new(
                1.,
                FlexBasis::Length((size::TAB_MIN * f64::from(scale)).px()),
                None,
            ))
        }),
        if app.writer.is_some() {
            xilem::view::FlexSpacer::Fixed(0.px())
        } else {
            xilem::view::FlexSpacer::Flex(1.)
        },
        (app.writer.is_some() && app.show_welcome() && !app.settings_open()).then(|| {
            print::action(scale, "Return to Writer", blocked, |app: &mut App| {
                app.return_to_document()
            })
        }),
        app.writer
            .as_ref()
            .filter(|writer| {
                !app.show_welcome()
                    && writer.navigation_width() == 0.
                    && !writer.attention.is_focused()
            })
            .map(|_| {
                print::action(scale, "Scenes…", blocked, |app: &mut App| {
                    app.open_picker(picker::Search::GoTo)
                })
            }),
        app.writer
            .as_ref()
            .filter(|_| !app.show_welcome())
            .map(|writer| {
                (
                    authoring_modes(writer, blocked),
                    print::action(scale, "Commands", blocked, |app: &mut App| {
                        app.open_picker(picker::Search::Commands)
                    }),
                )
            }),
        (!writing).then(|| {
            print::workspace(
                scale,
                (
                    app.settings_open(),
                    app.settings_open() || app.workspace_menu(),
                    held,
                ),
                |app: &mut App| app.toggle_workspace_menu(),
            )
        }),
    ))
    .gap(space::SIBLINGS.px())
}

/// Writing and localising are the app's two modes, chosen from the tab row.
fn authoring_modes(writer: &Writer, searching: bool) -> impl WidgetView<App> + use<> {
    let localise = writer.workspaces.screen == crate::workspaces::Screen::Localisation;
    tabs(
        "Mode",
        [("Write", false), ("Localise", true)].map(|(title, selected)| {
            tab(
                writer.ui_scale(),
                &writer.palette,
                (title, false),
                selected == localise,
                searching
                    || writer.project_action_blocked()
                    || (selected && writer.session.project().is_none()),
                move |app: &mut App| {
                    if let Some(writer) = &mut app.writer {
                        if selected {
                            writer.open_localisation();
                        } else {
                            writer.return_to_writing();
                        }
                    }
                },
            )
        }),
    )
}

fn workspace_actions(app: &App, scale: f32, searching: bool) -> impl WidgetView<App> + use<> {
    flex_row((
        app.writer.as_ref().map(|writer| {
            print::action(
                scale,
                "Projects",
                searching || writer.phase() != Phase::Writing,
                |app: &mut App| app.toggle_projects(),
            )
        }),
        print::action(
            scale,
            "Settings",
            searching
                || app.settings_open()
                || app
                    .writer
                    .as_ref()
                    .is_some_and(|writer| writer.phase() != Phase::Writing),
            |app: &mut App| app.settings_from_menu(),
        ),
        app.writer
            .as_ref()
            .filter(|_| !app.show_welcome() && !app.settings_open())
            .map(|writer| {
                print::action(
                    scale,
                    "Localise",
                    searching
                        || writer.project_action_blocked()
                        || writer.session.project().is_none(),
                    |app: &mut App| app.localise_from_menu(),
                )
            }),
        app.writer
            .as_ref()
            .filter(|_| !app.show_welcome() && !app.settings_open())
            .map(|_| search_buttons(app)),
    ))
    .direction(
        if app
            .writer
            .as_ref()
            .is_some_and(|writer| writer.viewport.width < 800. * f64::from(scale))
        {
            Axis::Vertical
        } else {
            Axis::Horizontal
        },
    )
    .gap(space::SIBLINGS.px())
}

fn search_buttons(app: &App) -> impl WidgetView<App> + use<> {
    let scale = crate::presentation::scale(&app.preferences.config.writer);
    let disabled = app.picker.is_some()
        || app
            .writer
            .as_ref()
            .is_none_or(|writer| writer.phase() != Phase::Writing || rename_modal(writer));
    flex_row((
        print::action(scale, "Go to", disabled, |app: &mut App| {
            app.search_from_menu(picker::Search::GoTo)
        }),
        app.writer
            .as_ref()
            .and_then(|writer| writer.session.project())
            .map(|_| {
                print::action(scale, "Find project text", disabled, |app: &mut App| {
                    app.search_from_menu(picker::Search::ProjectText)
                })
            }),
    ))
    .gap(space::SIBLINGS.px())
}
