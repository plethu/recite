//! Query input, scope selection and concise search guidance.
use super::*;

pub(super) fn scopes(app: &App, width: f64) -> impl WidgetView<App> + use<> {
    let scale = crate::presentation::scale(&app.preferences.config.writer);
    let mode = app.picker.as_ref().map(Picker::mode);
    radio_group(
        flex_row(
            [
                ("Scenes / beats", Search::GoTo, Mode::GoTo),
                ("Saved project", Search::ProjectText, Mode::ProjectText),
                ("This beat", Search::Passages, Mode::Passages),
            ]
            .map(|(title, search, selected)| {
                radio_button(
                    scale,
                    title,
                    mode == Some(selected),
                    move |app: &mut App| app.switch_search(search),
                )
                .disabled(
                    selected == Mode::ProjectText
                        && app
                            .writer
                            .as_ref()
                            .is_none_or(|writer| writer.session.project().is_none()),
                )
            }),
        )
        .direction(if width < 360. * f64::from(scale) {
            Axis::Vertical
        } else {
            Axis::Horizontal
        })
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px()),
    )
}

pub(super) fn heading(scale: f32, mode: Mode, width: f64) -> impl WidgetView<App> + use<> {
    let wide = width >= 300. * f64::from(scale);
    flex_row((
        label(scale, mode.title())
            .text_size(20. * scale)
            .prop(LineBreaking::WordWrap)
            .flex(if wide { 1. } else { 0. }),
        text_button(scale, "Close search", |app: &mut App| app.dismiss_picker()),
    ))
    .direction(if wide {
        Axis::Horizontal
    } else {
        Axis::Vertical
    })
    .width(width.px())
    .gap(8.px())
}

pub(super) fn query(
    scale: f32,
    mode: Mode,
    picker: Option<&Picker>,
    width: f64,
) -> impl WidgetView<App> + use<> {
    sized_box(
        text_input(
            picker.map_or_else(String::new, |picker| picker.query.clone()),
            |app: &mut App, value: String| app.filter_picker(value),
        )
        .placeholder(mode.title())
        .text_size(14. * scale)
        .on_enter(|app: &mut App, _| app.submit_picker())
        .on_cancel(|app: &mut App, _| app.dismiss_picker())
        .prop(TextAreaLabel::new("Search query"))
        .prop(QueryField(true)),
    )
    .width(width.px())
}
