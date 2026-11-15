use super::*;
use crate::presentation::segments::{segment, segments};
use crate::presentation::space;
use crate::presentation::{Type, Typeset};
use recite_config::{Keymap, WriterPaneSide, WriterPresentationField as Field, WriterTheme};
use xilem::view::checkbox;
use xilem::view::{GridExt, GridTrackSize, grid};
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
        .text_size(Type::Body.size(scale))
    })
    .collect::<Vec<_>>();
    flex_col((
        grid((
            specimen::specimen(app, sample_width).grid_pos(0, u16::from(!wide)),
            reading::reading(app, controls_width).grid_pos(u16::from(wide), 0),
        ))
        .columns(if wide {
            vec![
                GridTrackSize::Fixed(sample_width.px()),
                GridTrackSize::Fixed(controls_width.px()),
            ]
        } else {
            vec![GridTrackSize::Fixed(width.px())]
        })
        .rows(vec![GridTrackSize::Auto; if wide { 1 } else { 2 }])
        .gap(space::REGIONS.px()),
        label(scale, "Appearance").typeset(Type::Section, scale),
        choices(
            app,
            [
                (
                    "Light theme",
                    writer.theme == WriterTheme::Light,
                    UserConfigEdit::WriterTheme(WriterTheme::Light),
                ),
                (
                    "Dark theme",
                    writer.theme == WriterTheme::Dark,
                    UserConfigEdit::WriterTheme(WriterTheme::Dark),
                ),
            ],
        ),
        label(scale, "Editing").typeset(Type::Section, scale),
        choices(
            app,
            [
                (
                    "Standard keymap",
                    config.ui.keymap == Keymap::Standard,
                    UserConfigEdit::Keymap(Keymap::Standard),
                ),
                (
                    "Vim keymap",
                    config.ui.keymap == Keymap::Vim,
                    UserConfigEdit::Keymap(Keymap::Vim),
                ),
            ],
        ),
        choices(
            app,
            [
                (
                    "Script pane on the left",
                    writer.pane_side == WriterPaneSide::Left,
                    UserConfigEdit::WriterPaneSide(WriterPaneSide::Left),
                ),
                (
                    "Script pane on the right",
                    writer.pane_side == WriterPaneSide::Right,
                    UserConfigEdit::WriterPaneSide(WriterPaneSide::Right),
                ),
            ],
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
        .text_size(Type::Body.size(scale)),
        flex_col(booleans)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(space::SIBLINGS.px()),
        prose(
            scale,
            format!("Personal configuration: {}", app.preferences.path),
        ),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::TEXT.px())
}

/// A preference with two choices: one segmented track, one Tab stop.
fn choices(
    app: &App,
    options: [(&'static str, bool, UserConfigEdit); 2],
) -> impl WidgetView<App> + use<> {
    let scale = crate::presentation::scale(&app.preferences.config.writer);
    let palette = &app.theme.palette;
    segments(
        palette,
        options.map(|(title, chosen, edit)| {
            segment(
                scale,
                palette,
                title,
                chosen,
                false,
                move |app: &mut App| app.update_preference(edit.clone()),
            )
        }),
    )
}
