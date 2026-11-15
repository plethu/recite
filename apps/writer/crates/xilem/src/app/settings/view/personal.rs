use super::*;
use crate::presentation::segments::{segment, segments_along};
use crate::presentation::{Type, print, space};
use recite_config::{Keymap, WriterPaneSide, WriterPresentationField as Field, WriterTheme};
use xilem::masonry::kurbo::Axis;
use xilem::masonry::properties::LineBreaking;
use xilem::view::checkbox;
mod reading;

pub(super) fn personal(app: &App, width: f64) -> impl WidgetView<App> + use<> {
    let config = &app.preferences.config;
    let writer = &config.writer;
    let scale = crate::presentation::scale(writer);
    let presentation = writer.presentation;
    let check = |caption: &'static str, checked: bool, edit: fn(bool) -> UserConfigEdit| {
        checkbox(caption, checked, move |app: &mut App, value| {
            app.update_preference(edit(value))
        })
        .text_size(Type::Body.size(scale))
    };
    flex_col((
        group(scale, "Reading and layout", reading::reading(app, width)),
        group(
            scale,
            "Appearance",
            flex_col((
                choices(
                    app,
                    Axis::Horizontal,
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
                check(
                    "Monochrome colours",
                    writer.monochrome,
                    UserConfigEdit::WriterMonochrome,
                ),
                check(
                    "Reduce animation",
                    writer.reduced_motion,
                    UserConfigEdit::WriterReducedMotion,
                ),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(space::SIBLINGS.px()),
        ),
        group(
            scale,
            "Editing",
            flex_col((
                choices(
                    app,
                    Axis::Horizontal,
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
                // Two long choices stack on the sheet's narrow measure.
                choices(
                    app,
                    Axis::Vertical,
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
                check(
                    "Always show shortcut hints",
                    writer.shortcut_hints,
                    UserConfigEdit::WriterShortcutHints,
                ),
                check(
                    "Zoom around the pointer",
                    writer.zoom_to_pointer,
                    UserConfigEdit::WriterZoomToPointer,
                ),
                check(
                    "Confirm before closing",
                    writer.confirm_exit,
                    UserConfigEdit::WriterConfirmExit,
                ),
            ))
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(space::SIBLINGS.px()),
        ),
        group(
            scale,
            "Project",
            // A quiet row action, not a third choice beside the page segments.
            print::action(
                scale,
                "Project settings…",
                app.writer.as_ref().is_none_or(|writer| {
                    writer.session.project().is_none()
                        || writer.project_jobs_busy()
                        || writer.phase() != crate::Phase::Writing
                }),
                |app: &mut App| app.open_project_settings(),
            ),
        ),
        print::meta(
            scale,
            format!("Personal configuration: {}", app.preferences.path),
        )
        .color(app.theme.palette.muted)
        .prop(LineBreaking::WordWrap)
        .width(width.px()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::GROUPS.px())
}

/// A group of preferences under its Section head.
fn group<V: WidgetView<App>>(
    scale: f32,
    title: &'static str,
    rows: V,
) -> impl WidgetView<App> + use<V> {
    flex_col((Type::Section.label(scale, title), rows))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(space::TEXT.px())
}

/// A preference with two choices: one segmented track, one Tab stop.
fn choices(
    app: &App,
    axis: Axis,
    options: [(&'static str, bool, UserConfigEdit); 2],
) -> impl WidgetView<App> + use<> {
    let scale = crate::presentation::scale(&app.preferences.config.writer);
    let palette = &app.theme.palette;
    segments_along(
        palette,
        axis,
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
