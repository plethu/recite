//! The access form scrolls independently of its persistent submission actions.
use super::{App, ProjectField};
use crate::presentation::{identity, label, paragraph, print, radio_button, text_button};
use xilem::{
    WidgetView,
    masonry::{
        kurbo::Axis,
        layout::{AsUnit, Dim},
        properties::TextAreaLabel,
    },
    style::Style,
    view::{
        CrossAxisAlignment, FlexExt, FlexSpacer, MainAxisAlignment, flex_col, flex_row, portal,
        radio_group, resize_observer, sized_box, text_input,
    },
};

/// The welcome column: the hero wordmark, the form and its actions share one measure.
const COLUMN: f64 = 560.;

#[derive(Clone, Copy)]
struct Layout {
    scale: f32,
    width: f64,
    field_width: f64,
    direction: Axis,
}
impl Layout {
    fn resolve(app: &App) -> Self {
        let scale = crate::presentation::scale(&app.preferences.config.writer);
        let width = (app.viewport.width - 48.)
            .max(240.)
            .min(COLUMN * f64::from(scale));
        let wide = width >= 480. * f64::from(scale);
        Self {
            scale,
            width,
            field_width: if wide {
                width - 160. * f64::from(scale) - 8.
            } else {
                width
            },
            direction: if wide {
                Axis::Horizontal
            } else {
                Axis::Vertical
            },
        }
    }
}

pub(in crate::app) fn panel(app: &App) -> impl WidgetView<App> + use<> {
    let layout = Layout::resolve(app);
    let (form, surplus) = app.welcome_layout.flex();
    resize_observer(
        |app: &mut App, size| {
            if app.welcome_layout.available != Some(size) {
                app.welcome_layout.available = Some(size);
            }
        },
        sized_box(
            flex_col((
                portal(resize_observer(
                    |app: &mut App, size| {
                        if app.welcome_layout.content != Some(size) {
                            app.welcome_layout.content = Some(size);
                        }
                    },
                    column(layout, sized_box(fields(app, layout)).padding(24.px())),
                ))
                .constrain_horizontal(true)
                .flex(form),
                resize_observer(
                    |app: &mut App, size| {
                        if app.welcome_layout.actions != Some(size) {
                            app.welcome_layout.actions = Some(size);
                        }
                    },
                    column(layout, submission(app, layout)),
                ),
                FlexSpacer::Flex(surplus),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .gap(0.px()),
        )
        .height(Dim::Stretch),
    )
}

/// Centre a part of the welcome page on the column's measure.
fn column<V: WidgetView<App>>(layout: Layout, child: V) -> impl WidgetView<App> + use<V> {
    flex_row((sized_box(child).width((layout.width + 48.).px()),))
        .main_axis_alignment(MainAxisAlignment::Center)
}

fn fields(app: &App, layout: Layout) -> impl WidgetView<App> + use<> {
    let busy = app.opening.is_some() || app.auxiliary;
    let Layout {
        scale,
        width,
        direction,
        ..
    } = layout;
    flex_col((
        identity::hero(&app.theme, width.min(360. * f64::from(scale))),
        label(scale, "Projects").text_size(24. * scale),
        paragraph(
            scale,
            "Open a project or create one to start writing.",
            width,
        ),
        radio_group(
            flex_row((
                radio_button(
                    scale,
                    "Open an existing project",
                    !app.creating,
                    |app: &mut App| app.choose_project_action(false),
                )
                .disabled(busy),
                radio_button(
                    scale,
                    "Create a new project",
                    app.creating,
                    |app: &mut App| app.choose_project_action(true),
                )
                .disabled(busy),
            ))
            .direction(direction)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(12.px()),
        ),
        folder(app, layout),
        app.creating.then(|| name(app, layout)),
        super::feedback::details(app, scale, width),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
}

fn folder(app: &App, layout: Layout) -> impl WidgetView<App> + use<> {
    let busy = app.opening.is_some() || app.auxiliary;
    let Layout {
        scale,
        field_width,
        direction,
        ..
    } = layout;
    flex_col((
        label(
            scale,
            if app.creating {
                "New project folder"
            } else {
                "Project folder"
            },
        ),
        flex_row((
            text_input(app.path.clone(), |app: &mut App, text| {
                app.edit_project_path(text)
            })
            .placeholder("Project folder")
            .text_size(14. * scale)
            .disabled(busy)
            .prop(TextAreaLabel::new("Project folder"))
            .prop(ProjectField(true))
            .width(field_width.px()),
            text_button(scale, "Browse folders", |app: &mut App| app.browse()).disabled(busy),
        ))
        .direction(direction)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
}

fn name(app: &App, layout: Layout) -> impl WidgetView<App> + use<> {
    let Layout {
        scale,
        width,
        field_width,
        ..
    } = layout;
    flex_col((
        label(scale, "Project name"),
        text_input(app.name.clone(), |app: &mut App, text| app.edit_project_name(text))
            .placeholder("Project name").text_size(14. * scale)
            .disabled(app.opening.is_some() || app.auxiliary)
            .prop(TextAreaLabel::new("Project name")).prop(ProjectField(true))
            .width(field_width.px()),
        paragraph(scale, "Creates recite.project.toml and a first scene with a default beat. Existing project files are preserved.", width),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(8.px())
}

fn submission(app: &App, layout: Layout) -> impl WidgetView<App> + use<> {
    let Layout {
        scale,
        width,
        direction,
        ..
    } = layout;
    let busy = app.opening.is_some() || app.auxiliary;
    sized_box(
        flex_col((
            super::feedback::panel(app, layout.scale, layout.width),
            (app.welcome_problem.is_none() && !app.status.is_empty()).then(|| {
                portal(paragraph(scale, app.status.clone(), width - 16.))
                    .constrain_horizontal(true)
                    .height((56. * f64::from(scale)).px())
            }),
            flex_row((
                print::primary_action(
                    scale,
                    if app.creating {
                        "Create project"
                    } else {
                        "Open selected project"
                    },
                    &app.theme.palette,
                    !app.project_form_ready(),
                    |app: &mut App| app.open_selected(),
                ),
                app.opening.as_ref().map(|_| {
                    text_button(scale, "Cancel opening", |app: &mut App| app.cancel_open())
                }),
                // One primary; the examples are a quiet way in. Closing belongs to the window.
                print::action(scale, "Try temporary examples", busy, |app: &mut App| {
                    app.show_examples()
                }),
            ))
            .direction(direction)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(8.px()),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px()),
    )
    .padding(xilem::style::Padding::from_vh(12.px(), 24.px()))
}
