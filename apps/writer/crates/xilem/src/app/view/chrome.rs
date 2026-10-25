//! Window identity and workspace commands live outside the manuscript's reading measure.
use crate::presentation::{identity, label, paragraph, print};
use crate::{
    Phase, Writer,
    app::{App, picker},
};
use xilem::{
    WidgetView,
    masonry::{kurbo::Axis, layout::AsUnit, parley::style::FontWeight},
    style::Style,
    view::{CrossAxisAlignment, flex_col, flex_row, sized_box},
};
pub(super) fn app_bar(app: &App) -> impl WidgetView<App> + use<> {
    let scale = crate::presentation::scale(&app.preferences.config.writer);
    let searching = app.picker.is_some();
    let blocked = searching
        || app
            .writer
            .as_ref()
            .is_some_and(|writer| writer.phase != Phase::Writing);
    sized_box(
        flex_col((
            identity_bar(app, scale, blocked),
            app.workspace_menu
                .then(|| workspace_actions(app, scale, searching)),
            (!app.status.is_empty() && (!app.show_welcome || app.settings_open))
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
        .gap(8.px()),
    )
    .padding(xilem::style::Padding::from_vh(4.px(), 12.px()))
    .background_color(app.theme.palette.surface)
}

fn identity_bar(app: &App, scale: f32, blocked: bool) -> impl WidgetView<App> + use<> {
    let writing = !app.show_welcome
        && !app.settings_open
        && app.writer.as_ref().is_some_and(|writer| {
            writer.workspaces.screen == crate::workspaces::Screen::Write
                && !writer.attention.is_focused()
        });
    flex_row((
        sized_box(flex_row((
            identity::wordmark(&app.theme, scale),
            xilem::view::FlexSpacer::Flex(1.),
        )))
        .width(app.writer.as_ref().map_or(220., Writer::drawer_width).px()),
        app.writer
            .as_ref()
            .filter(|writer| writer.viewport.width >= 1100. * f64::from(scale))
            .map(|writer| {
                flex_col((
                    sized_box(
                        label(scale, writer.session.model().document().key().to_string())
                            .weight(FontWeight::SEMI_BOLD)
                            .color(app.theme.palette.ink),
                    )
                    .padding(xilem::style::Padding::from_vh(4.px(), 8.px())),
                    flex_col(())
                        .height(1.px())
                        .background_color(app.theme.palette.teal),
                ))
                .cross_axis_alignment(CrossAxisAlignment::Stretch)
                .gap(0.px())
            }),
        xilem::view::FlexSpacer::Flex(1.),
        app.writer
            .as_ref()
            .filter(|writer| {
                !writing
                    && !app.show_welcome
                    && !app.settings_open
                    && writer.session.project().is_some()
                    && writer.viewport.width >= 1100. * f64::from(scale)
            })
            .map(|writer| {
                print::action(
                    scale,
                    "Open in editor",
                    blocked || writer.project_action_blocked() || writer.external.busy(),
                    |app: &mut App| {
                        if let Some(writer) = &mut app.writer {
                            writer.open_editor();
                        }
                    },
                )
            }),
        (app.writer.is_some() && app.show_welcome && !app.settings_open).then(|| {
            print::action(scale, "Return to Writer", blocked, |app: &mut App| {
                app.show_welcome = false
            })
        }),
        app.settings_open.then(|| {
            print::action(scale, "Done with settings", blocked, |app: &mut App| {
                app.settings_open = false
            })
        }),
        app.writer
            .as_ref()
            .filter(|writer| {
                !app.show_welcome && !app.settings_open && writer.navigation_width() == 0.
            })
            .map(|_| {
                print::action(scale, "Scenes…", blocked, |app: &mut App| {
                    app.open_picker(picker::Search::GoTo)
                })
            }),
        app.writer
            .as_ref()
            .filter(|_| !writing && !app.show_welcome && !app.settings_open)
            .map(|_| {
                print::action(scale, "Commands", blocked, |app: &mut App| {
                    app.open_picker(picker::Search::Commands)
                })
            }),
        (!writing).then(|| {
            print::action(scale, "Workspace", blocked, |app: &mut App| {
                app.workspace_menu = !app.workspace_menu
            })
        }),
    ))
    .gap(8.px())
}

fn workspace_actions(app: &App, scale: f32, searching: bool) -> impl WidgetView<App> + use<> {
    flex_row((
        app.writer.as_ref().map(|writer| {
            print::action(
                scale,
                "Projects",
                searching || writer.phase != Phase::Writing,
                |app: &mut App| {
                    app.show_welcome = !app.show_welcome;
                    app.settings_open = false;
                    app.workspace_menu = false;
                },
            )
        }),
        print::action(
            scale,
            "Settings",
            searching
                || app.settings_open
                || app
                    .writer
                    .as_ref()
                    .is_some_and(|writer| writer.phase != Phase::Writing),
            |app: &mut App| {
                app.workspace_menu = false;
                app.show_settings();
            },
        ),
        app.writer
            .as_ref()
            .filter(|_| !app.show_welcome && !app.settings_open)
            .map(|writer| {
                print::action(
                    scale,
                    "Localize",
                    searching
                        || writer.project_action_blocked()
                        || writer.session.project().is_none(),
                    |app: &mut App| {
                        app.workspace_menu = false;
                        if let Some(writer) = &mut app.writer {
                            writer.open_localisation();
                        }
                    },
                )
            }),
        app.writer
            .as_ref()
            .filter(|_| !app.show_welcome && !app.settings_open)
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
    .gap(8.px())
}

fn search_buttons(app: &App) -> impl WidgetView<App> + use<> {
    let scale = crate::presentation::scale(&app.preferences.config.writer);
    let disabled = app.picker.is_some()
        || app
            .writer
            .as_ref()
            .is_none_or(|writer| writer.phase != Phase::Writing);
    flex_row((
        print::action(scale, "Go to", disabled, |app: &mut App| {
            app.workspace_menu = false;
            app.open_picker(picker::Search::GoTo)
        }),
        app.writer
            .as_ref()
            .and_then(|writer| writer.session.project())
            .map(|_| {
                print::action(scale, "Find project text", disabled, |app: &mut App| {
                    app.workspace_menu = false;
                    app.open_picker(picker::Search::ProjectText)
                })
            }),
    ))
    .gap(8.px())
}
