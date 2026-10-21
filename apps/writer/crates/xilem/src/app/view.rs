mod chrome;
use super::*;
use crate::presentation::{label, prose, radio_button, text_button};
use chrome::app_bar;
use xilem::masonry::properties;
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
    if let Some(writer) = &mut app.writer {
        writer.viewport.width = app.viewport.width;
    }
    let slot = slot::writer_slot(app);
    keyboard::host(
        app,
        xilem::core::fork(
            xilem::view::resize_observer(
                |app: &mut App, size| app.viewport = size,
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
                .gap(0.px())
                .background_color(app.theme.palette.canvas),
            ),
            opening::worker(),
        ),
    )
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
            sized_box(text_input(app.path.clone(), |app: &mut App, text| { app.path = text; app.native_path = None; }).placeholder("Project folder").text_size(14. * scale).disabled(busy).prop(properties::TextAreaLabel::new("Project folder"))).width(600.px()),
            text_button(scale, "Browse folders", App::browse).disabled(busy),
        )).gap(8.px()),
        app.creating.then(|| flex_col((
            label(scale, "Project name"),
            sized_box(text_input(app.name.clone(), |app: &mut App, text| app.name = text).placeholder("Project name").text_size(14. * scale).disabled(busy).prop(properties::TextAreaLabel::new("Project name"))).width(600.px()),
            prose(scale, "Creates recite.project.toml and a first scene with a default beat. Existing project files are preserved."),
        )).cross_axis_alignment(CrossAxisAlignment::Start).gap(8.px())),
        flex_row((
            text_button(scale, if app.creating { "Create project" } else { "Open selected project" }, App::open_selected).disabled(busy || app.path.trim().is_empty() || (app.creating && app.name.trim().is_empty())),
            app.opening.as_ref().map(|_| text_button(scale, "Cancel opening", App::cancel_open)),
            text_button(scale, "Close Writer", App::request_close).disabled(app.auxiliary),
        )).gap(8.px()),
    )).cross_axis_alignment(CrossAxisAlignment::Start).gap(12.px())).padding(24.px())).constrain_horizontal(true)
}
