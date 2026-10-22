use super::*;
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
        .gap(32.px())
}

fn commands(app: &App, width: f64) -> impl WidgetView<App> + use<> {
    let scale = crate::presentation::scale(&app.preferences.config.writer);
    let count = app.matching_shortcuts().count();
    let page = app.settings.page;
    let pages = count.div_ceil(COMMANDS_PER_PAGE).max(1);
    let choices = app
        .matching_shortcuts()
        .skip(page * COMMANDS_PER_PAGE)
        .take(COMMANDS_PER_PAGE)
        .map(|command| {
            let binding = app.preferences.config.writer.shortcuts.binding(command);
            radio_button(
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
                app.settings.command == command,
                move |app: &mut App| app.choose_shortcut(command),
            )
        })
        .collect::<Vec<_>>();
    flex_col((
        label(scale, "Find a command").text_size(18. * scale),
        text_input(app.settings.query.clone(), App::search_shortcuts)
            .text_size(14. * scale)
            .placeholder("Search shortcuts")
            .prop(TextAreaLabel::new("Search shortcuts"))
            .width(width.min(600.).px()),
        label(scale, format!("{count} matching commands")),
        radio_group(
            flex_col(choices)
                .cross_axis_alignment(CrossAxisAlignment::Start)
                .gap(8.px()),
        ),
        (pages > 1).then(|| {
            flex_row((
                text_button(scale, "Previous commands", |app: &mut App| {
                    app.settings.page = app.settings.page.saturating_sub(1)
                })
                .disabled(page == 0),
                label(scale, format!("Commands {} of {pages}", page + 1)),
                text_button(scale, "Next commands", |app: &mut App| {
                    app.settings.page += 1
                })
                .disabled(page + 1 >= pages),
            ))
            .gap(8.px())
        }),
    ))
    .cross_axis_alignment(CrossAxisAlignment::Start)
    .gap(12.px())
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
        label(scale, format!("Shortcut for {selected}")).text_size(18. * scale),
        label(scale, format!("Saved binding: {}", if stored.is_empty() { "Disabled" } else { stored })),
        sized_box(keyed(app.settings.command, text_input(app.shortcut_draft().to_owned(), App::edit_shortcut)
            .text_size(14. * scale).prop(TextAreaLabel::new("Shortcut binding")))).width(width.min(600.).px()),
        flex_row((
            text_button(scale, "Save shortcut", App::save_shortcut).disabled(!changed),
            text_button(scale, "Reset shortcuts", App::reset_shortcuts),
            changed.then(|| label(scale, "Unsaved binding")),
        )).gap(8.px()),
        paragraph(scale, if cfg!(target_os = "macos") { "Primary means Command. Leave empty to disable. Apply, Undo and Redo belong to the focused editor." } else { "Primary means Ctrl. Leave empty to disable. Apply, Undo and Redo belong to the focused editor." }, width),
        app.settings.feedback.as_ref().map(|feedback| paragraph(scale, feedback.clone(), width)),
    )).cross_axis_alignment(CrossAxisAlignment::Start).gap(8.px()).width(width.px())
}
