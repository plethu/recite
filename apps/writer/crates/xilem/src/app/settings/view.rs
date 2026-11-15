use super::*;
use crate::presentation::segments::{segment, segments};
use crate::presentation::space;
use crate::presentation::{Type, Typeset};
use crate::presentation::{label, prose, text_button};
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
                label(scale, "Settings").typeset(Type::Display, scale),
                navigation(app),
                indexed_stack((personal(app), keyboard(app)))
                    .active(usize::from(app.settings.keyboard)),
                app.preferences
                    .error
                    .as_ref()
                    .map(|error| prose(scale, error.clone())),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(space::KINDS.px()),
        )
        .padding(space::GROUPS.px()),
    )
    .constrain_horizontal(true)
    // Settings is a page, not the desk: it keeps a flat ground beneath its forms and prose.
    .background_color(app.theme.palette.canvas)
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
        segments(
            &app.theme.palette,
            [
                ("Personal preferences", false),
                ("Keyboard shortcuts", true),
            ]
            .map(|(title, keys)| {
                segment(
                    scale,
                    &app.theme.palette,
                    title,
                    app.settings.keyboard == keys,
                    false,
                    move |app: &mut App| app.settings.keyboard = keys,
                )
            }),
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
    .gap(space::TEXT.px())
}
