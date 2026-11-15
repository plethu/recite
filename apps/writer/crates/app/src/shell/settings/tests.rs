use super::*;

#[test]
fn unfinished_shortcuts_survive_switching_commands_and_reopening_settings() {
    let mut app = Shell::empty();
    app.choose_shortcut(WriterCommand::Save);
    app.edit_shortcut("Primary+Alt+S".into());
    app.choose_shortcut(WriterCommand::SaveAll);
    let save_all = app.shortcut_draft().to_owned();
    app.show_settings();
    assert_eq!(app.shortcut_draft(), save_all);
    app.choose_shortcut(WriterCommand::Save);
    assert_eq!(app.shortcut_draft(), "Primary+Alt+S");
    app.show_settings();
    assert_eq!(app.shortcut_draft(), "Primary+Alt+S");
    app.choose_shortcut(WriterCommand::Source);
    app.search_shortcuts("save".into());
    app.show_settings();
    assert!(
        app.matching_shortcuts()
            .all(|command| command != WriterCommand::Source)
    );
    app.choose_shortcut(WriterCommand::Save);
    assert_eq!(app.shortcut_draft(), "Primary+Alt+S");
}

#[test]
fn conflicting_or_failed_shortcut_saves_retain_the_field_and_the_saved_binding() {
    let mut app = Shell::empty();
    app.choose_shortcut(WriterCommand::Save);
    let saved = app
        .preferences
        .config
        .writer
        .shortcuts
        .binding(WriterCommand::Save)
        .to_owned();
    app.edit_shortcut("Primary+Shift+S".into());
    app.save_shortcut();
    assert_eq!(
        app.preferences
            .config
            .writer
            .shortcuts
            .binding(WriterCommand::Save),
        saved
    );
    assert_eq!(app.shortcut_draft(), "Primary+Shift+S");
    assert!(
        app.settings
            .feedback
            .as_ref()
            .unwrap()
            .contains("not saved")
    );
}

#[test]
fn presentation_values_are_set_within_bounds_on_their_step_and_steps_stop_at_the_ends() {
    let mut app = Shell::empty();
    for field in [
        WriterPresentationField::ReadingSize,
        WriterPresentationField::UiScale,
        WriterPresentationField::ScriptWidth,
    ] {
        let (min, max) = field.bounds();
        app.set_presentation(field, u16::MAX);
        assert_eq!(app.preferences.config.writer.presentation.value(field), max);
        app.adjust_presentation(field, true);
        assert_eq!(app.preferences.config.writer.presentation.value(field), max);
        app.set_presentation(field, 0);
        assert_eq!(app.preferences.config.writer.presentation.value(field), min);
        app.set_presentation(field, min + presentation_step(field) * 3 / 2);
        assert_eq!(
            app.preferences.config.writer.presentation.value(field),
            min + presentation_step(field)
        );
        app.set_presentation(field, min);
        app.adjust_presentation(field, true);
        assert_eq!(
            app.preferences.config.writer.presentation.value(field),
            min + presentation_step(field)
        );
    }
}
