use super::*;
use xilem::{
    WidgetView,
    masonry::layout::AsUnit,
    style::Style,
    view::{
        CrossAxisAlignment, FlexExt, flex_col, flex_row, indexed_stack, label, portal, prose,
        radio_button, radio_group, sized_box, text_button, text_input,
    },
};
pub(super) fn app(app: &mut App) -> impl WidgetView<App> + use<> {
    let slot = slot::writer_slot(app);
    xilem::core::fork(
        flex_col((
            sized_box(
                flex_row((
                    app.writer.as_ref().map(|writer| {
                        text_button("Projects", |app: &mut App| {
                            app.show_welcome = !app.show_welcome
                        })
                        .disabled(writer.phase != Phase::Writing)
                    }),
                    app.writer.as_ref().filter(|_| app.show_welcome).map(|_| {
                        text_button("Return to Writer", |app: &mut App| app.show_welcome = false)
                    }),
                    label(app.status.clone()),
                ))
                .gap(12.px()),
            )
            .padding(12.px()),
            indexed_stack((welcome(app), slot))
                .active(usize::from(!app.show_welcome))
                .flex(1.),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Stretch)
        .background_color(xilem::peniko::Color::from_rgb8(27, 24, 30)),
        opening::worker(),
    )
}
fn welcome(app: &App) -> impl WidgetView<App> + use<> {
    let busy = app.opening.is_some() || app.auxiliary;
    portal(sized_box(flex_col((
        label("Recite Writer").text_size(32.),
        prose("A place to write dialogue, inspect its structure, and try conversations."),
        flex_row((
            radio_group(flex_row((
                radio_button("Open an existing project", !app.creating, |app: &mut App| app.creating = false).disabled(busy),
                radio_button("Create a new project", app.creating, |app: &mut App| app.creating = true).disabled(busy),
            )).gap(12.px())),
            text_button("Try temporary examples", App::show_examples).disabled(busy),
        )).gap(8.px()),
        label(if app.creating { "New project folder" } else { "Project folder" }),
        flex_row((
            sized_box(text_input(app.path.clone(), |app: &mut App, text| { app.path = text; app.native_path = None; }).placeholder("Project folder").disabled(busy).prop(xilem::masonry::properties::TextAreaLabel::new("Project folder"))).width(600.px()),
            text_button("Browse folders", App::browse).disabled(busy),
        )).gap(8.px()),
        app.creating.then(|| flex_col((
            label("Project name"),
            sized_box(text_input(app.name.clone(), |app: &mut App, text| app.name = text).placeholder("Project name").disabled(busy).prop(xilem::masonry::properties::TextAreaLabel::new("Project name"))).width(600.px()),
            prose("Creates recite.project.toml and a first scene with a default beat. Existing project files are preserved."),
        )).cross_axis_alignment(CrossAxisAlignment::Start).gap(8.px())),
        flex_row((
            text_button(if app.creating { "Create project" } else { "Open selected project" }, App::open_selected).disabled(busy || app.path.trim().is_empty() || (app.creating && app.name.trim().is_empty())),
            app.opening.as_ref().map(|_| text_button("Cancel opening", App::cancel_open)),
            text_button("Close Writer", App::request_close).disabled(app.auxiliary),
        )).gap(8.px()),
    )).cross_axis_alignment(CrossAxisAlignment::Start).gap(12.px())).padding(24.px())).constrain_horizontal(true)
}
