use super::*;
use recite_config::{Keymap, WriterPaneSide, WriterPresentationField as Field, WriterTheme};
use xilem::view::checkbox;

pub(super) fn personal(app: &App) -> impl WidgetView<App> + use<> {
    let config = &app.preferences.config;
    let writer = &config.writer;
    let scale = crate::presentation::scale(writer);
    let presentation = writer.presentation;
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
        flex_col((
            label(
                scale,
                format!(
                    "{caption}: {}{}",
                    presentation.value(field),
                    if field == Field::UiScale { "%" } else { "" }
                ),
            ),
            flex_row((
                text_button(
                    scale,
                    format!("Decrease {caption}"),
                    move |app: &mut App| app.adjust_presentation(field, false),
                )
                .disabled(presentation.value(field) == min),
                text_button(
                    scale,
                    format!("Increase {caption}"),
                    move |app: &mut App| app.adjust_presentation(field, true),
                )
                .disabled(presentation.value(field) == max),
            ))
            .gap(8.px()),
        ))
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(8.px())
    })
    .collect::<Vec<_>>();
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
        text_button(scale, "Project settings…", App::open_project_settings).disabled(
            app.writer.as_ref().is_none_or(|writer| {
                writer.session.project().is_none()
                    || writer.project_jobs_busy()
                    || writer.phase != crate::Phase::Writing
            }),
        ),
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
        flex_col(fields)
            .cross_axis_alignment(CrossAxisAlignment::Start)
            .gap(8.px()),
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
