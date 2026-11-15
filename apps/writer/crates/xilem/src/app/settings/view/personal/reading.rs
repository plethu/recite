use super::*;
use crate::presentation::group::named;
use crate::presentation::space;
use crate::presentation::{Icon, print};
use crate::presentation::{Type, Typeset};
use xilem::masonry::properties::LineBreaking;

pub(super) fn reading(app: &App, width: f64) -> impl WidgetView<App> + use<> {
    let presentation = app.preferences.config.writer.presentation;
    let scale = crate::presentation::scale(&app.preferences.config.writer);
    let wide = width >= 300. * f64::from(scale);
    let fields = [
        (
            "Reading text size",
            Field::ReadingSize,
            "Smaller reading text",
            "Larger reading text",
        ),
        (
            "Source text size",
            Field::SourceSize,
            "Smaller source text",
            "Larger source text",
        ),
        (
            "UI scale",
            Field::UiScale,
            "Decrease UI scale",
            "Increase UI scale",
        ),
        (
            "Navigation width",
            Field::DrawerWidth,
            "Narrower navigation",
            "Wider navigation",
        ),
        (
            "Script width",
            Field::ScriptWidth,
            "Narrower script",
            "Wider script",
        ),
    ]
    .into_iter()
    .map(|(caption, field, decrease, increase)| {
        let (min, max) = field.bounds();
        named(
            caption,
            flex_row((
                label(
                    scale,
                    format!(
                        "{caption}: {}{}",
                        presentation.value(field),
                        if field == Field::UiScale { "%" } else { " px" }
                    ),
                )
                .prop(LineBreaking::WordWrap)
                .width((190. * f64::from(scale)).min(width).px()),
                flex_row((
                    print::icon_action(
                        scale,
                        decrease,
                        Icon::Minus,
                        &app.theme.palette,
                        presentation.value(field) == min,
                        move |app: &mut App| app.adjust_presentation(field, false),
                    ),
                    print::icon_action(
                        scale,
                        increase,
                        Icon::Plus,
                        &app.theme.palette,
                        presentation.value(field) == max,
                        move |app: &mut App| app.adjust_presentation(field, true),
                    ),
                ))
                .gap(space::SIBLINGS.px()),
            ))
            .direction(if wide {
                Axis::Horizontal
            } else {
                Axis::Vertical
            })
            .cross_axis_alignment(CrossAxisAlignment::Center)
            .gap(space::SIBLINGS.px())
            .width(width.px()),
        )
    })
    .collect::<Vec<_>>();
    flex_col((
        label(scale, "Reading and layout").typeset(Type::Section, scale),
        flex_col(fields)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(space::SIBLINGS.px()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::TEXT.px())
    .width(width.px())
}
