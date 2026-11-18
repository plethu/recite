//! The query field and the scope segment.
use super::*;
use crate::presentation::segments::{segment, segments_along};
use crate::presentation::tokens::breakpoint;
use crate::presentation::{Type, Typeset, search_field};
use xilem::peniko::Color;

pub(super) fn scopes(app: &App, width: f64) -> impl WidgetView<App> + use<> {
    let scale = crate::presentation::scale(&app.preferences.config.writer);
    let mode = app.picker.as_ref().map(Picker::mode);
    // One segmented track, stacked where the picker is too narrow for a row.
    segments_along(
        &app.theme.palette,
        if width < breakpoint::SEGMENTS * f64::from(scale) {
            Axis::Vertical
        } else {
            Axis::Horizontal
        },
        [
            ("Scenes / beats", Search::GoTo, Mode::GoTo),
            ("Saved project", Search::ProjectText, Mode::ProjectText),
            ("This beat", Search::Passages, Mode::Passages),
        ]
        .map(|(title, search, selected)| {
            segment(
                scale,
                &app.theme.palette,
                title,
                mode == Some(selected),
                selected == Mode::ProjectText
                    && app
                        .writer
                        .as_ref()
                        .is_none_or(|writer| writer.session.project().is_none()),
                move |app: &mut App| app.switch_search(search),
            )
        }),
    )
}

pub(super) fn query(
    scale: f32,
    picker: Option<&Picker>,
    width: f64,
    ink: Color,
) -> impl WidgetView<App> + use<> {
    search_field(
        scale,
        ink,
        false,
        // The close action shares the line.
        Some(width - print::icon_action_width(scale) - space::SIBLINGS),
        text_input(
            picker.map_or_else(String::new, |picker| picker.query.clone()),
            |app: &mut App, value: String| app.filter_picker(value),
        )
        .placeholder(picker.map_or(Mode::Commands, Picker::mode).placeholder())
        .typeset(Type::Body, scale)
        .on_enter(|app: &mut App, _| app.submit_picker())
        .on_cancel(|app: &mut App, _| app.dismiss_picker())
        .prop(TextAreaLabel::new("Search query"))
        .prop(QueryField(true)),
    )
}
