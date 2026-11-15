//! The access form scrolls independently of its persistent submission actions.
use super::{App, ProjectField};
use crate::presentation::control_style::{ControlStyle, styled};
use crate::presentation::segments::{tab, tabs};
use crate::presentation::slip::slip;
use crate::presentation::space;
use crate::presentation::tokens::breakpoint;
use crate::presentation::{Type, Typeset, field_text};
use crate::presentation::{identity, label, paragraph, print, text_button};
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
        resize_observer, sized_box, text_input,
    },
};

/// The welcome column: the hero wordmark, the form and its actions share one measure.
const COLUMN: f64 = 560.;
/// The Browse folders button's width beside the folder field.
const BROWSE: f64 = 160.;
/// The narrowest column, below which the page scrolls sideways rather than crush its fields.
const NARROWEST: f64 = 240.;
/// The hero wordmark's widest.
const HERO: f64 = 360.;
/// The status note's height: three Body lines before it scrolls.
const STATUS_LINES: f64 = 3.;

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
        let width = (app.viewport.width - 2. * space::GROUPS)
            .max(NARROWEST)
            .min(COLUMN * f64::from(scale));
        let wide = width >= breakpoint::FIELD_ROW * f64::from(scale);
        Self {
            scale,
            width,
            // Inside the slip's padding, beside Browse folders where the column is wide.
            field_width: if wide {
                width - BROWSE * f64::from(scale) - space::SIBLINGS - 2. * space::KINDS
            } else {
                width - 2. * space::KINDS
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
                    column(
                        layout,
                        sized_box(fields(app, layout)).padding(space::GROUPS.px()),
                    ),
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
    flex_row((sized_box(child).width((layout.width + 2. * space::GROUPS).px()),))
        .main_axis_alignment(MainAxisAlignment::Center)
}

fn fields(app: &App, layout: Layout) -> impl WidgetView<App> + use<> {
    let busy = app.opening.is_some() || app.auxiliary;
    let Layout { scale, width, .. } = layout;
    flex_col((
        identity::hero(&app.theme, width.min(HERO * f64::from(scale))),
        label(scale, "Projects").typeset(Type::Display, scale),
        paragraph(
            scale,
            "Open a project or create one to start writing.",
            width,
        ),
        // Opening and creating are two tabs over one compact slip of fields.
        tabs((
            tab(
                scale,
                &app.theme.palette,
                "Open an existing project",
                !app.creating,
                busy,
                |app: &mut App| app.choose_project_action(false),
            ),
            tab(
                scale,
                &app.theme.palette,
                "Create a new project",
                app.creating,
                busy,
                |app: &mut App| app.choose_project_action(true),
            ),
        )),
        slip(
            &app.theme.palette,
            sized_box(
                flex_col((folder(app, layout), app.creating.then(|| name(app, layout))))
                    .cross_axis_alignment(CrossAxisAlignment::Start)
                    .gap(space::TEXT.px()),
            )
            .padding(space::KINDS.px()),
        ),
        super::feedback::details(app, scale, width),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::TEXT.px())
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
            styled(
                ControlStyle::DeskField,
                text_input(app.path.clone(), |app: &mut App, text| {
                    app.edit_project_path(text)
                })
                .text_size(field_text(scale))
                .disabled(busy)
                .prop(TextAreaLabel::new("Project folder"))
                .prop(ProjectField(true))
                .width(field_width.px()),
            ),
            text_button(scale, "Browse folders", |app: &mut App| app.browse()).disabled(busy),
        ))
        .direction(direction)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(space::SIBLINGS.px()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::UNIT.px())
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
        styled(ControlStyle::DeskField, text_input(app.name.clone(), |app: &mut App, text| app.edit_project_name(text))
            .text_size(field_text(scale))
            .disabled(app.opening.is_some() || app.auxiliary)
            .prop(TextAreaLabel::new("Project name")).prop(ProjectField(true))
            .width(field_width.px())),
        paragraph(scale, "Creates recite.project.toml and a first scene with a default beat. Existing project files are preserved.", width),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::UNIT.px())
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
                portal(paragraph(scale, app.status.clone(), width - space::KINDS))
                    .constrain_horizontal(true)
                    .height((STATUS_LINES * f64::from(Type::Body.metrics().1 * scale)).px())
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
            .gap(space::SIBLINGS.px()),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(space::SIBLINGS.px()),
    )
    .padding(xilem::style::Padding::from_vh(
        space::KINDS.px(),
        space::GROUPS.px(),
    ))
}
