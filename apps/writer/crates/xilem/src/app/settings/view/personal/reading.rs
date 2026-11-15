use super::*;
use crate::presentation::number::{Range, Spin, number_field};
use crate::presentation::space;

pub(super) fn reading(app: &App, width: f64) -> impl WidgetView<App> + use<> {
    let presentation = app.preferences.config.writer.presentation;
    let scale = crate::presentation::scale(&app.preferences.config.writer);
    let fields = [
        ("Reading text size", Field::ReadingSize),
        ("Source text size", Field::SourceSize),
        ("UI scale", Field::UiScale),
        ("Navigation width", Field::DrawerWidth),
        ("Script width", Field::ScriptWidth),
    ]
    .into_iter()
    .map(|(caption, field)| {
        let (min, max) = field.bounds();
        let value = presentation.value(field);
        super::row(
            scale,
            width,
            caption,
            number_field(
                scale,
                caption,
                format!(
                    "{value}{}",
                    if field == Field::UiScale { "%" } else { " px" }
                ),
                Range {
                    value: f64::from(value),
                    min: f64::from(min),
                    max: f64::from(max),
                    step: f64::from(presentation_step(field)),
                },
                app.theme.palette.ink,
                move |app: &mut App, spin| match spin {
                    Spin::Step { up } => app.adjust_presentation(field, up),
                    // The field keeps the value within the range; the app keeps it on its step.
                    Spin::To(target) => app.set_presentation(field, target.round() as u16),
                },
            ),
        )
    })
    .collect::<Vec<_>>();
    flex_col(fields)
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(space::SIBLINGS.px())
}
