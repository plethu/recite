use recite_config::{
    Platform, PlatformRoots, UserConfigEdit, UserConfigStore, WriterCommand as C, WriterShortcut,
    WriterShortcuts, resolve_config_path,
};

#[test]
fn conflicts_include_defaults_and_failed_edits_preserve_the_previous_binding() {
    let mut bindings = WriterShortcuts::default();
    let before = bindings.clone();
    let error = bindings
        .rebind(
            C::Commands,
            WriterShortcut::try_from("Primary+S".to_owned()).unwrap(),
        )
        .unwrap_err();
    assert!(error.to_string().contains("save"));
    assert_eq!(bindings, before);
    bindings.rebind(C::Save, WriterShortcut::default()).unwrap();
    bindings
        .rebind(
            C::Commands,
            WriterShortcut::try_from("Primary+S".to_owned()).unwrap(),
        )
        .unwrap();
    assert_eq!(bindings.binding(C::Save), "");
    assert_eq!(bindings.binding(C::Commands), "Primary+S");
}

#[test]
fn text_entry_and_reserved_keys_cannot_be_stolen_by_workspace_bindings() {
    for key in [
        "K",
        "Shift+K",
        "Primary+Z",
        "Primary+C",
        "Primary+Q",
        "F6",
        "F13",
        "F01",
        "+F8",
        "Primary++F8",
        "Primary+",
        "Alt+F4",
        "Primary+Primary+K",
        "Shift+Primary+K",
    ] {
        assert!(WriterShortcut::try_from(key.to_owned()).is_err(), "{key}");
    }
    for key in ["", "F8", "Primary+K", "Primary+Shift+K", "Primary+Alt+1"] {
        assert!(WriterShortcut::try_from(key.to_owned()).is_ok(), "{key}");
    }
}

#[test]
fn access_preferences_and_shortcuts_round_trip_and_invalid_files_are_rejected()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("user.toml");
    std::fs::write(
        &path,
        "# personal settings\nconfig_version = 1\n[ui]\nlocale = 'en-GB' # keep\n",
    )?;
    let store = UserConfigStore::new(resolve_config_path(
        Platform::Linux,
        &PlatformRoots::new(),
        Some(&path),
    )?);
    let mut bindings = WriterShortcuts::default();
    bindings.rebind(C::Commands, WriterShortcut::try_from("F8".to_owned())?)?;
    store.update(UserConfigEdit::WriterShortcuts(bindings.clone()))?;
    store.update(UserConfigEdit::WriterMonochrome(true))?;
    store.update(UserConfigEdit::WriterShortcutHints(true))?;
    let loaded = store.load()?;
    assert_eq!(loaded.config.writer.shortcuts, bindings);
    assert!(loaded.config.writer.monochrome && loaded.config.writer.shortcut_hints);
    assert!(std::fs::read_to_string(&path)?.contains("locale = 'en-GB' # keep"));
    for invalid in [
        "commands = 'Primary+S'",
        "commands = 'K'",
        "commands = 'F8'\nsave = 'F8'",
        "apply = 'F8'",
        "unknown = 'F8'",
    ] {
        std::fs::write(
            &path,
            format!("config_version = 1\n[writer.shortcuts]\n{invalid}\n"),
        )?;
        assert!(store.load().is_err(), "{invalid}");
    }
    Ok(())
}
