use super::*;
use crate::presentation::control_style::{ControlStyle, styled};
use crate::presentation::print::section_heading;
use crate::presentation::segments::{segment_named, segments_on_paper};
use crate::presentation::tokens::size;
use crate::presentation::{Type, Typeset, print, space};
use recite_config::{Keymap, WriterPaneSide, WriterPresentationField as Field, WriterTheme};
use xilem::masonry::properties::LineBreaking;
use xilem::view::{FlexExt, checkbox};
mod reading;

pub(super) fn personal(app: &App, width: f64) -> impl WidgetView<App> + use<> {
    let config = &app.preferences.config;
    let writer = &config.writer;
    let scale = crate::presentation::scale(writer);
    let presentation = writer.presentation;
    let check = |caption: &'static str, checked: bool, edit: fn(bool) -> UserConfigEdit| {
        check_row(
            scale,
            width,
            caption,
            checked,
            move |app: &mut App, value| app.update_preference(edit(value)),
        )
    };
    flex_col((
        group(scale, "Reading and layout", reading::reading(app, width)),
        group(
            scale,
            "Appearance",
            flex_col((
                choices(
                    app,
                    width,
                    "Theme",
                    [
                        (
                            ("Light", "Light theme"),
                            writer.theme == WriterTheme::Light,
                            UserConfigEdit::WriterTheme(WriterTheme::Light),
                        ),
                        (
                            ("Dark", "Dark theme"),
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
                    width,
                    "Keymap",
                    [
                        (
                            ("Standard", "Standard keymap"),
                            config.ui.keymap == Keymap::Standard,
                            UserConfigEdit::Keymap(Keymap::Standard),
                        ),
                        (
                            ("Vim", "Vim keymap"),
                            config.ui.keymap == Keymap::Vim,
                            UserConfigEdit::Keymap(Keymap::Vim),
                        ),
                    ],
                ),
                choices(
                    app,
                    width,
                    "Script pane",
                    [
                        (
                            ("Left", "Script pane on the left"),
                            writer.pane_side == WriterPaneSide::Left,
                            UserConfigEdit::WriterPaneSide(WriterPaneSide::Left),
                        ),
                        (
                            ("Right", "Script pane on the right"),
                            writer.pane_side == WriterPaneSide::Right,
                            UserConfigEdit::WriterPaneSide(WriterPaneSide::Right),
                        ),
                    ],
                ),
                check_row(
                    scale,
                    width,
                    "Split map and script",
                    presentation.split(),
                    move |app: &mut App, value| {
                        app.update_preference(UserConfigEdit::WriterPresentation(
                            presentation.with_split(value),
                        ))
                    },
                ),
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
    flex_col((section_heading(scale, title), rows))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(space::TEXT.px())
}

/// One preference: its label on the left and its control against the sheet's right edge, on a
/// row's height so every preference keeps one rhythm.
pub(super) fn row<V: WidgetView<App>>(
    scale: f32,
    width: f64,
    caption: &'static str,
    control: V,
) -> impl WidgetView<App> + use<V> {
    flex_row((
        Type::Body
            .label(scale, caption)
            .prop(LineBreaking::WordWrap)
            .flex(1.),
        control,
    ))
    .cross_axis_alignment(CrossAxisAlignment::Center)
    .gap(space::SIBLINGS.px())
    .width(width.px())
}

/// A checkbox across the whole row, so its hover and its target span the row.
fn check_row<F: Fn(&mut App, bool) + Send + Sync + 'static>(
    scale: f32,
    width: f64,
    caption: &'static str,
    checked: bool,
    edit: F,
) -> impl WidgetView<App> + use<F> {
    styled(
        ControlStyle::Flush,
        checkbox(caption, checked, edit)
            .typeset(Type::Body, scale)
            .dims((width.px(), (size::ROW * f64::from(scale)).px())),
    )
}

/// A preference with two choices, labelled on the left: one compact segmented track on the
/// right, one Tab stop, each choice read by its full name.
fn choices(
    app: &App,
    width: f64,
    caption: &'static str,
    options: [((&'static str, &'static str), bool, UserConfigEdit); 2],
) -> impl WidgetView<App> + use<> {
    let scale = crate::presentation::scale(&app.preferences.config.writer);
    let palette = &app.theme.palette;
    row(
        scale,
        width,
        caption,
        segments_on_paper(
            palette,
            options.map(|(title, chosen, edit)| {
                segment_named(scale, palette, title, chosen, move |app: &mut App| {
                    app.update_preference(edit.clone())
                })
            }),
        ),
    )
}
