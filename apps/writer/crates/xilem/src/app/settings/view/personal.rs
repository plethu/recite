use super::*;
use recite_config::{Keymap, WriterPaneSide, WriterPresentationField as Field, WriterTheme};
use xilem::masonry::kurbo::Axis;
use xilem::view::checkbox;
mod reading;
mod specimen;

pub(super) fn personal(app: &App) -> impl WidgetView<App> + use<> {
    let config = &app.preferences.config;
    let writer = &config.writer;
    let scale = crate::presentation::scale(writer);
    let presentation = writer.presentation;
    let width = (app.viewport.width - 48.).max(280.);
    let wide = width >= 1000. * f64::from(scale);
    let sample_width = if wide { width * 0.38 } else { width };
    let controls_width = if wide {
        width - sample_width - 32.
    } else {
        width
    };
    let booleans = [
        (
            "Monochrome colours",
            writer.monochrome,
            UserConfigEdit::WriterMonochrome as fn(bool) -> UserConfigEdit,
        ),
        (
            "Reduce animation",
            writer.reduced_motion,
            UserConfigEdit::WriterReducedMotion,
        ),
        (
            "Always show shortcut hints",
            writer.shortcut_hints,
            UserConfigEdit::WriterShortcutHints,
        ),
        (
            "Zoom around the pointer",
            writer.zoom_to_pointer,
            UserConfigEdit::WriterZoomToPointer,
        ),
        (
            "Confirm before closing",
            writer.confirm_exit,
            UserConfigEdit::WriterConfirmExit,
        ),
    ]
    .into_iter()
    .map(|(caption, checked, edit)| {
        checkbox(caption, checked, move |app: &mut App, value| {
            app.update_preference(edit(value))
        })
        .text_size(14. * scale)
    })
    .collect::<Vec<_>>();
    flex_col((
        flex_row((
            specimen::specimen(app, sample_width),
            reading::reading(app, controls_width),
        ))
        .direction(if wide {
            Axis::Horizontal
        } else {
            Axis::Vertical
        })
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(32.px()),
        label(scale, "Appearance").text_size(18. * scale),
        radio_group(
            flex_row((
                radio_button(
                    scale,
                    "Light theme",
                    writer.theme == WriterTheme::Light,
                    |app: &mut App| {
                        app.update_preference(UserConfigEdit::WriterTheme(WriterTheme::Light))
                    },
                ),
                radio_button(
                    scale,
                    "Dark theme",
                    writer.theme == WriterTheme::Dark,
                    |app: &mut App| {
                        app.update_preference(UserConfigEdit::WriterTheme(WriterTheme::Dark))
                    },
                ),
            ))
            .gap(12.px()),
        ),
        label(scale, "Editing").text_size(18. * scale),
        radio_group(
            flex_row((
                radio_button(
                    scale,
                    "Standard keymap",
                    config.ui.keymap == Keymap::Standard,
                    |app: &mut App| app.update_preference(UserConfigEdit::Keymap(Keymap::Standard)),
                ),
                radio_button(
                    scale,
                    "Vim keymap",
                    config.ui.keymap == Keymap::Vim,
                    |app: &mut App| app.update_preference(UserConfigEdit::Keymap(Keymap::Vim)),
                ),
            ))
            .gap(12.px()),
        ),
        radio_group(
            flex_row((
                radio_button(
                    scale,
                    "Script pane on the left",
                    writer.pane_side == WriterPaneSide::Left,
                    |app: &mut App| {
                        app.update_preference(UserConfigEdit::WriterPaneSide(WriterPaneSide::Left))
                    },
                ),
                radio_button(
                    scale,
                    "Script pane on the right",
                    writer.pane_side == WriterPaneSide::Right,
                    |app: &mut App| {
                        app.update_preference(UserConfigEdit::WriterPaneSide(WriterPaneSide::Right))
                    },
                ),
            ))
            .gap(12.px()),
        ),
        checkbox(
            "Split map and script",
            presentation.split(),
            move |app: &mut App, value| {
                app.update_preference(UserConfigEdit::WriterPresentation(
                    presentation.with_split(value),
                ))
            },
        )
        .text_size(14. * scale),
        flex_col(booleans)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(8.px()),
        prose(
            scale,
            format!("Personal configuration: {}", app.preferences.path),
        ),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
}
