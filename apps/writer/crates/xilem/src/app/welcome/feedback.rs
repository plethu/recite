//! Project problem views over `recite_writer_app::shell::welcome::feedback`.
use super::super::App;
use crate::presentation::{group, label, paragraph, text_button};
use xilem::{
    WidgetView,
    masonry::layout::AsUnit,
    style::Style,
    view::{CrossAxisAlignment, flex_col},
};

pub(super) fn panel(app: &App, scale: f32, width: f64) -> Option<impl WidgetView<App> + use<>> {
    let problem = app.welcome_problem.as_ref()?;
    Some(group::status(
        problem.heading(),
        flex_col((
            label(scale, problem.heading())
                .text_size(16. * scale)
                .color(app.theme.palette.accent),
            paragraph(scale, problem.guidance(), width),
            text_button(
                scale,
                if problem.expanded {
                    "Hide project error details"
                } else {
                    "Show project error details"
                },
                |app: &mut App| {
                    if let Some(problem) = &mut app.welcome_problem {
                        problem.expanded = !problem.expanded;
                    }
                },
            ),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px()),
    ))
}

pub(super) fn details(app: &App, scale: f32, width: f64) -> Option<impl WidgetView<App> + use<>> {
    let problem = app
        .welcome_problem
        .as_ref()
        .filter(|problem| problem.expanded)?;
    Some(group::named(
        "Project error details",
        flex_col((
            label(scale, "Error details").color(app.theme.palette.muted),
            paragraph(scale, problem.diagnostic.clone(), width),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px()),
    ))
}
