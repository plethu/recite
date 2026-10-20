use super::*;
use crate::keyed::keyed;
use xilem::{masonry::properties::TextAreaLabel, view::text_input};

pub(super) fn keyboard(app: &App) -> impl WidgetView<App> + use<> {
    let scale = crate::presentation::scale(&app.preferences.config.writer);
    let count = commands().count();
    let page = app.settings.page;
    let pages = count.div_ceil(COMMANDS_PER_PAGE);
    let choices = commands()
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
    let stored = app
        .preferences
        .config
        .writer
        .shortcuts
        .binding(app.settings.command);
    let changed = app.shortcut_draft() != stored;
    let selected = command_label(app.settings.command);
    flex_col((
        prose(scale, if cfg!(target_os = "macos") { "Primary means Command on this system. Use Primary+Shift+S, for example, or leave the field empty to disable a shortcut. Apply, Undo and Redo belong to the focused editor." } else { "Primary means Ctrl on this system. Use Primary+Shift+S, for example, or leave the field empty to disable a shortcut. Apply, Undo and Redo belong to the focused editor." }),
        radio_group(flex_col(choices).cross_axis_alignment(CrossAxisAlignment::Start).gap(8.px())),
        flex_row((
            text_button(scale, "Previous commands", |app: &mut App| app.settings.page = app.settings.page.saturating_sub(1)).disabled(page == 0),
            label(scale, format!("Commands {} of {pages}", page + 1)),
            text_button(scale, "Next commands", |app: &mut App| app.settings.page += 1).disabled(page + 1 >= pages),
        )).gap(8.px()),
        label(scale, format!("Shortcut for {selected}")).text_size(18. * scale),
        label(scale, format!("Saved binding: {}", if stored.is_empty() { "Disabled" } else { stored })),
        sized_box(keyed(app.settings.command, text_input(app.shortcut_draft().to_owned(), App::edit_shortcut).text_size(14. * scale).prop(TextAreaLabel::new("Shortcut binding")))).width(600.px()),
        changed.then(|| label(scale, format!("Unsaved shortcut for {selected}."))),
        app.settings.feedback.as_ref().map(|feedback| prose(scale, feedback.clone())),
        flex_row((
            text_button(scale, "Save shortcut", App::save_shortcut).disabled(!changed),
            text_button(scale, "Reset shortcuts", App::reset_shortcuts),
        )).gap(8.px()),
    )).cross_axis_alignment(CrossAxisAlignment::Start).gap(12.px())
}
