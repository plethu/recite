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
    assert_eq!(app.settings.page, 0);
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
