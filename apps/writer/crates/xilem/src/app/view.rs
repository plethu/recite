use super::*;
use crate::presentation::{label, paragraph, prose, radio_button, text_button};
use xilem::{
    WidgetView,
    masonry::layout::AsUnit,
    style::Style,
    view::{
        CrossAxisAlignment, FlexExt, flex_col, flex_row, indexed_stack, portal, radio_group,
        sized_box, text_input,
    },
};
pub(super) fn app(app: &mut App) -> impl WidgetView<App> + use<> {
    let slot = slot::writer_slot(app);
    keyboard::host(
        app,
        xilem::core::fork(
            flex_col((
                app_bar(app),
                indexed_stack((
                    welcome(app),
                    slot,
                    settings::settings(app),
                    picker::view::picker(app),
                ))
                .active(if app.picker.is_some() {
                    3
                } else if app.settings_open {
                    2
                } else {
                    usize::from(!app.show_welcome)
                })
                .flex(1.),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .background_color(app.theme.palette.canvas),
            opening::worker(),
        ),
    )
}
fn app_bar(app: &App) -> impl WidgetView<App> + use<> {
    let scale = crate::presentation::scale(&app.preferences.config.writer);
    let searching = app.picker.is_some();
    sized_box(
        flex_col((
            flex_row((
                app.writer.as_ref().map(|writer| {
                    text_button(scale, "Projects", |app: &mut App| {
                        app.show_welcome = !app.show_welcome;
                        app.settings_open = false;
                    })
                    .disabled(searching || writer.phase != Phase::Writing)
                }),
                app.writer
                    .as_ref()
                    .filter(|_| app.show_welcome && !app.settings_open)
                    .map(|_| {
                        text_button(scale, "Return to Writer", |app: &mut App| {
                            app.show_welcome = false
                        })
                    }),
                text_button(scale, "Settings", App::show_settings).disabled(
                    searching
                        || app.settings_open
                        || app
                            .writer
                            .as_ref()
                            .is_some_and(|writer| writer.phase != Phase::Writing),
                ),
                app.writer
                    .as_ref()
                    .filter(|_| !app.show_welcome && !app.settings_open)
                    .map(|_| search_buttons(app)),
                app.settings_open.then(|| {
                    text_button(scale, "Done with settings", |app: &mut App| {
                        app.settings_open = false
                    })
                }),
            ))
            .gap(12.px()),
            (!app.status.is_empty()).then(|| paragraph(scale, app.status.clone(), 800.)),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px()),
    )
    .padding(12.px())
}

fn search_buttons(app: &App) -> impl WidgetView<App> + use<> {
    let scale = crate::presentation::scale(&app.preferences.config.writer);
    let disabled = app.picker.is_some()
        || app
            .writer
            .as_ref()
            .is_none_or(|writer| writer.phase != Phase::Writing);
    flex_row((
        text_button(scale, "Commands", |app: &mut App| {
            app.open_picker(picker::Mode::Commands)
        })
        .disabled(disabled),
        text_button(scale, "Go to", |app: &mut App| {
            app.open_picker(picker::Mode::GoTo)
        })
        .disabled(disabled),
        app.writer
            .as_ref()
            .and_then(|writer| writer.session.project())
            .map(|_| {
                text_button(scale, "Find project text", |app: &mut App| {
                    app.open_picker(picker::Mode::ProjectText)
                })
                .disabled(disabled)
            }),
    ))
    .gap(8.px())
}

fn welcome(app: &App) -> impl WidgetView<App> + use<> {
    let scale = crate::presentation::scale(&app.preferences.config.writer);
    let busy = app.opening.is_some() || app.auxiliary;
    portal(sized_box(flex_col((
        label(scale, "Recite Writer").text_size(32. * scale),
        prose(scale, "A place to write dialogue, inspect its structure, and try conversations."),
        flex_row((
            radio_group(flex_row((
                radio_button(scale, "Open an existing project", !app.creating, |app: &mut App| app.creating = false).disabled(busy),
                radio_button(scale, "Create a new project", app.creating, |app: &mut App| app.creating = true).disabled(busy),
            )).gap(12.px())),
            text_button(scale, "Try temporary examples", App::show_examples).disabled(busy),
        )).gap(8.px()),
        label(scale, if app.creating { "New project folder" } else { "Project folder" }),
        flex_row((
            sized_box(text_input(app.path.clone(), |app: &mut App, text| { app.path = text; app.native_path = None; }).placeholder("Project folder").text_size(14. * scale).disabled(busy).prop(xilem::masonry::properties::TextAreaLabel::new("Project folder"))).width(600.px()),
            text_button(scale, "Browse folders", App::browse).disabled(busy),
        )).gap(8.px()),
        app.creating.then(|| flex_col((
            label(scale, "Project name"),
            sized_box(text_input(app.name.clone(), |app: &mut App, text| app.name = text).placeholder("Project name").text_size(14. * scale).disabled(busy).prop(xilem::masonry::properties::TextAreaLabel::new("Project name"))).width(600.px()),
            prose(scale, "Creates recite.project.toml and a first scene with a default beat. Existing project files are preserved."),
        )).cross_axis_alignment(CrossAxisAlignment::Start).gap(8.px())),
        flex_row((
            text_button(scale, if app.creating { "Create project" } else { "Open selected project" }, App::open_selected).disabled(busy || app.path.trim().is_empty() || (app.creating && app.name.trim().is_empty())),
            app.opening.as_ref().map(|_| text_button(scale, "Cancel opening", App::cancel_open)),
            text_button(scale, "Close Writer", App::request_close).disabled(app.auxiliary),
        )).gap(8.px()),
    )).cross_axis_alignment(CrossAxisAlignment::Start).gap(12.px())).padding(24.px())).constrain_horizontal(true)
}
