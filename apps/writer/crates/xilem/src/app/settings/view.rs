use super::*;
use crate::presentation::{label, prose, radio_button, text_button};
use xilem::{
    WidgetView,
    masonry::{kurbo::Axis, layout::AsUnit},
    style::Style,
    view::{CrossAxisAlignment, flex_col, flex_row, indexed_stack, portal, radio_group, sized_box},
};

pub(in crate::app) fn settings(app: &App) -> impl WidgetView<App> + use<> {
    let scale = crate::presentation::scale(&app.preferences.config.writer);
    portal(
        sized_box(
            flex_col((
                label(scale, "Settings").text_size(28. * scale),
                navigation(app),
                indexed_stack((personal(app), keyboard(app)))
                    .active(usize::from(app.settings.keyboard)),
                app.preferences
                    .error
                    .as_ref()
                    .map(|error| prose(scale, error.clone())),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(16.px()),
        )
        .padding(24.px()),
    )
    .constrain_horizontal(true)
}
mod keyboard;
mod personal;
use keyboard::keyboard;
use personal::personal;

#[cfg(test)]
mod tests;

fn navigation(app: &App) -> impl WidgetView<App> + use<> {
    let scale = crate::presentation::scale(&app.preferences.config.writer);
    flex_row((
        radio_group(
            flex_row((
                radio_button(
                    scale,
                    "Personal preferences",
                    !app.settings.keyboard,
                    |app: &mut App| app.settings.keyboard = false,
                ),
                radio_button(
                    scale,
                    "Keyboard shortcuts",
                    app.settings.keyboard,
                    |app: &mut App| app.settings.keyboard = true,
                ),
            ))
            .gap(12.px()),
        ),
        text_button(scale, "Project settings…", |app: &mut App| {
            app.open_project_settings()
        })
        .disabled(app.writer.as_ref().is_none_or(|writer| {
            writer.session.project().is_none()
                || writer.project_jobs_busy()
                || writer.phase() != crate::Phase::Writing
        })),
    ))
    .direction(if app.viewport.width >= 600. * f64::from(scale) {
        Axis::Horizontal
    } else {
        Axis::Vertical
    })
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
}
