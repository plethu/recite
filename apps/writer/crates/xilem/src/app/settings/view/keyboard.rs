use super::*;
use crate::presentation::row::{listed, row};
use crate::presentation::space;
use crate::presentation::{Type, Typeset, field_text, search_field};
use crate::{keyed::keyed, presentation::paragraph};
use xilem::{
    masonry::{kurbo::Axis, properties::TextAreaLabel},
    view::text_input,
};

pub(super) fn keyboard(app: &App) -> impl WidgetView<App> + use<> {
    let scale = crate::presentation::scale(&app.preferences.config.writer);
    let width = (app.viewport.width - 48.).max(280.);
    let wide = width >= 560. * f64::from(scale);
    let column_width = if wide { (width - 32.) / 2. } else { width };
    flex_row((editor(app, column_width), commands(app, column_width)))
        .direction(if wide {
            Axis::Horizontal
        } else {
            Axis::Vertical
        })
        .cross_axis_alignment(CrossAxisAlignment::Start)
        .gap(space::REGIONS.px())
}

fn commands(app: &App, width: f64) -> impl WidgetView<App> + use<> {
    let scale = crate::presentation::scale(&app.preferences.config.writer);
    let count = app.matching_shortcuts().count();
    // Every match is one row; the page scrolls, so the list needs no paging.
    let choices = app
        .matching_shortcuts()
        .map(|command| {
            let binding = app.preferences.config.writer.shortcuts.binding(command);
            let selected = app.settings.command == command;
            listed(
                &app.theme.palette,
                selected,
                row(
                    selected,
                    label(
                        scale,
                        format!(
                            "{} · {}",
                            command_label(command),
                            if binding.is_empty() {
                                "Disabled"
                            } else {
                                binding
                            }
                        ),
                    )
                    .typeset(Type::Control, scale),
                    move |app: &mut App| app.choose_shortcut(command),
                ),
            )
        })
        .collect::<Vec<_>>();
    flex_col((
        label(scale, "Find a command").typeset(Type::Section, scale),
        search_field(
            scale,
            app.theme.palette.muted,
            false,
            Some(width.min(600.)),
            text_input(
                app.settings.query.clone(),
                |app: &mut App, value: String| app.search_shortcuts(value),
            )
            .text_size(field_text(scale))
            .prop(TextAreaLabel::new("Search shortcuts")),
        ),
        label(scale, format!("{count} matching commands")),
        flex_col(choices)
            .cross_axis_alignment(CrossAxisAlignment::Stretch)
            .gap(0.px())
            .width(width.min(600.).px()),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(space::TEXT.px())
    .width(width.px())
}

fn editor(app: &App, width: f64) -> impl WidgetView<App> + use<> {
    let scale = crate::presentation::scale(&app.preferences.config.writer);
    let stored = app
        .preferences
        .config
        .writer
        .shortcuts
        .binding(app.settings.command);
    let changed = app.shortcut_draft() != stored;
    let selected = command_label(app.settings.command);
    flex_col((
        label(scale, format!("Shortcut for {selected}")).typeset(Type::Section, scale),
        label(scale, format!("Saved binding: {}", if stored.is_empty() { "Disabled" } else { stored })),
        sized_box(keyed(app.settings.command, text_input(app.shortcut_draft().to_owned(), |app: &mut App, value: String| app.edit_shortcut(value))
            .text_size(field_text(scale)).prop(TextAreaLabel::new("Shortcut binding")))).width(width.min(600.).px()),
        flex_row((
            text_button(scale, "Save shortcut", |app: &mut App| app.save_shortcut()).disabled(!changed),
            text_button(scale, "Reset shortcuts", |app: &mut App| app.reset_shortcuts()),
            changed.then(|| label(scale, "Unsaved binding")),
        )).gap(space::SIBLINGS.px()),
        paragraph(scale, if cfg!(target_os = "macos") { "Primary means Command. Leave empty to disable. Apply, Undo and Redo belong to the focused editor." } else { "Primary means Ctrl. Leave empty to disable. Apply, Undo and Redo belong to the focused editor." }, width),
        app.settings.feedback.as_ref().map(|feedback| paragraph(scale, feedback.clone(), width)),
    )).cross_axis_alignment(CrossAxisAlignment::Start).gap(space::SIBLINGS.px()).width(width.px())
}
