use recite_config::{UserConfigEdit, UserConfigStore, resolve_config_path};
fn update_exit(path: &std::path::Path, confirm: bool) -> Result<(), Box<dyn std::error::Error>> {
    let resolved = resolve_config_path(Platform::Linux, &PlatformRoots::new(), Some(path))?;
    UserConfigStore::new(resolved).update(UserConfigEdit::WriterConfirmExit(confirm))?;
    Ok(())
}
use recite_config::{Platform, PlatformRoots, load_user_config_from};

#[test]
fn exit_preference_round_trips_without_rewriting_other_settings()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("config.toml");
    std::fs::write(
        &path,
        "# My preferences\nconfig_version = 1\n[ui]\nlocale = 'en-GB' # keep this\n[play]\nshow_unavailable_choices = false\n",
    )?;
    update_exit(&path, false)?;
    let bytes = std::fs::read_to_string(&path)?;
    assert!(bytes.contains("# My preferences"));
    assert!(bytes.contains("locale = 'en-GB' # keep this"));
    let loaded = load_user_config_from(Platform::Linux, &PlatformRoots::new(), Some(&path))?;
    assert!(!loaded.config.writer.confirm_exit);
    assert!(!loaded.config.play.show_unavailable_choices);
    update_exit(&path, true)?;
    assert!(
        load_user_config_from(Platform::Linux, &PlatformRoots::new(), Some(&path))?
            .config
            .writer
            .confirm_exit
    );
    Ok(())
}

#[test]
fn new_config_is_created_only_by_explicit_write() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let roots = PlatformRoots::new().with_xdg_config_home(root.path());
    let loaded = load_user_config_from(Platform::Linux, &roots, None)?;
    assert!(loaded.config.writer.confirm_exit);
    let path = loaded.path.ok_or("config path")?;
    assert!(!path.exists());
    UserConfigStore::new(resolve_config_path(Platform::Linux, &roots, None)?)
        .update(UserConfigEdit::WriterConfirmExit(false))?;
    assert!(
        !load_user_config_from(Platform::Linux, &roots, None)?
            .config
            .writer
            .confirm_exit
    );
    Ok(())
}

#[test]
fn malformed_and_future_configs_are_preserved() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("config.toml");
    for source in [
        "config_version = 999\n",
        "[writer]\nconfirm_exit = 'no'\n",
        "[unknown]\nvalue = true\n",
    ] {
        std::fs::write(&path, source)?;
        assert!(update_exit(&path, false).is_err());
        assert_eq!(std::fs::read_to_string(&path)?, source);
    }
    Ok(())
}

#[test]
fn concurrent_preference_writer_is_refused() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("config.toml");
    std::fs::write(&path, "config_version = 1\n")?;
    let lock = std::fs::File::create(path.with_extension("toml.lock"))?;
    lock.try_lock()?;
    let store = UserConfigStore::new(resolve_config_path(
        Platform::Linux,
        &PlatformRoots::new(),
        Some(&path),
    )?);
    assert!(matches!(
        store.update(UserConfigEdit::WriterConfirmExit(false)),
        Err(recite_config::ConfigWriteError::Locked { .. })
    ));
    assert_eq!(std::fs::read_to_string(&path)?, "config_version = 1\n");
    Ok(())
}

#[cfg(unix)]
#[test]
fn symlink_config_is_not_replaced() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let target = root.path().join("original.toml");
    let link = root.path().join("config.toml");
    std::fs::write(&target, "config_version = 1\n")?;
    std::os::unix::fs::symlink(&target, &link)?;
    assert!(update_exit(&link, false).is_err());
    assert_eq!(std::fs::read_to_string(target)?, "config_version = 1\n");
    assert!(std::fs::symlink_metadata(link)?.is_symlink());
    Ok(())
}

#[test]
fn typed_edits_reload_external_changes_and_keep_field_provenance()
-> Result<(), Box<dyn std::error::Error>> {
    use recite_config::{
        ConfigAuthority, FieldProvenance, InvocationOverrides, Keymap, UserConfigField,
        resolve_user_config,
    };
    let root = tempfile::tempdir()?;
    let path = root.path().join("config.toml");
    std::fs::write(&path, "config_version = 1\n")?;
    let store = UserConfigStore::new(resolve_config_path(
        Platform::Linux,
        &PlatformRoots::new(),
        Some(&path),
    )?);
    let initial = store.load()?;
    assert!(!initial.field_is_explicit(UserConfigField::WriterConfirmExit));
    std::fs::write(
        &path,
        "config_version = 1\n[ui]\nlocale = 'cy' # external edit\n",
    )?;
    store.update(UserConfigEdit::Keymap(Keymap::Vim))?;
    let updated = store.update(UserConfigEdit::WriterConfirmExit(true))?;
    assert!(std::fs::read_to_string(path)?.contains("locale = 'cy' # external edit"));
    assert_eq!(updated.config.ui.keymap, Keymap::Vim);
    assert!(updated.field_is_explicit(UserConfigField::WriterConfirmExit));
    let resolved = resolve_user_config(&updated, &InvocationOverrides::new());
    assert_eq!(
        resolved.writer_confirm_exit().provenance(),
        FieldProvenance::Authority(ConfigAuthority::User)
    );
    Ok(())
}

#[test]
fn writer_exit_policy_refuses_project_and_invocation_authority() {
    use recite_config::{AuthorityValue, ConfigAuthority, WriterConfirmExitPolicy, resolve_field};
    for authority in [
        ConfigAuthority::Project,
        ConfigAuthority::Generated,
        ConfigAuthority::Invocation,
    ] {
        assert!(
            resolve_field(
                WriterConfirmExitPolicy,
                true,
                [AuthorityValue::new(authority, false)]
            )
            .is_err()
        );
    }
}

#[test]
fn missing_explicit_override_is_not_created_by_update() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("missing.toml");
    let store = UserConfigStore::new(resolve_config_path(
        Platform::Linux,
        &PlatformRoots::new(),
        Some(&path),
    )?);
    assert!(matches!(
        store.update(UserConfigEdit::WriterConfirmExit(false)),
        Err(recite_config::ConfigWriteError::Config(
            recite_config::ConfigError::MissingExplicit { .. }
        ))
    ));
    assert!(!path.exists());
    Ok(())
}

#[test]
fn shared_edits_handle_existing_preferences_and_inline_tables()
-> Result<(), Box<dyn std::error::Error>> {
    use recite_config::{KeyHints, Keymap, TuiColorMode, TuiContrast};
    let root = tempfile::tempdir()?;
    let path = root.path().join("config.toml");
    std::fs::write(
        &path,
        "config_version = 1\nui = { locale = 'en-US' } # keep inline\n",
    )?;
    let store = UserConfigStore::new(resolve_config_path(
        Platform::Linux,
        &PlatformRoots::new(),
        Some(&path),
    )?);
    for edit in [
        UserConfigEdit::UiLocale(recite_config::UiLocale::parse("cy")?),
        UserConfigEdit::Keymap(Keymap::Vim),
        UserConfigEdit::KeyHints(KeyHints::Hidden),
        UserConfigEdit::Color(TuiColorMode::Never),
        UserConfigEdit::Contrast(TuiContrast::Accessible),
        UserConfigEdit::ShowUnavailableChoices(false),
    ] {
        store.update(edit)?;
    }
    let loaded = store.load()?;
    assert_eq!(loaded.config.ui.locale.to_string(), "cy");
    assert_eq!(loaded.config.ui.keymap, Keymap::Vim);
    assert_eq!(loaded.config.ui.key_hints, KeyHints::Hidden);
    assert_eq!(loaded.config.ui.color, TuiColorMode::Never);
    assert_eq!(loaded.config.ui.contrast, TuiContrast::Accessible);
    assert!(!loaded.config.play.show_unavailable_choices);
    assert!(loaded.config.writer.confirm_exit);
    assert!(std::fs::read_to_string(path)?.contains("# keep inline"));
    Ok(())
}

#[test]
fn writer_workspace_preferences_round_trip_with_provenance()
-> Result<(), Box<dyn std::error::Error>> {
    use recite_config::{UserConfigField, WriterPaneSide, WriterTheme, WriterView};
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("config.toml");
    std::fs::write(&path, "config_version = 1\n")?;
    let store = UserConfigStore::new(resolve_config_path(
        Platform::Linux,
        &PlatformRoots::new(),
        Some(&path),
    )?);
    store.update(UserConfigEdit::WriterView(WriterView::Source))?;
    store.update(UserConfigEdit::WriterPaneSide(WriterPaneSide::Left))?;
    store.update(UserConfigEdit::WriterTheme(WriterTheme::Dark))?;
    store.update(UserConfigEdit::WriterReducedMotion(true))?;
    store.update(UserConfigEdit::WriterZoomToPointer(false))?;
    store.update(UserConfigEdit::Keymap(recite_config::Keymap::Vim))?;
    let reopened = store.load()?;
    assert_eq!(reopened.config.writer.view, WriterView::Source);
    assert_eq!(reopened.config.writer.pane_side, WriterPaneSide::Left);
    assert!(reopened.field_is_explicit(UserConfigField::WriterPaneSide));
    assert_eq!(reopened.config.writer.theme, WriterTheme::Dark);
    assert!(reopened.config.writer.reduced_motion);
    assert!(!reopened.config.writer.zoom_to_pointer);
    assert!(reopened.field_is_explicit(UserConfigField::WriterView));
    let resolved =
        recite_config::resolve_user_config(&reopened, &recite_config::InvocationOverrides::new());
    assert_eq!(*resolved.writer_view().value(), WriterView::Source);
    assert_eq!(*resolved.writer_pane_side().value(), WriterPaneSide::Left);
    assert_eq!(*resolved.ui().keymap().value(), recite_config::Keymap::Vim);
    Ok(())
}

#[test]
fn pane_side_defaults_to_right_and_rejects_invalid_or_project_owned_values()
-> Result<(), Box<dyn std::error::Error>> {
    use recite_config::{
        AuthorityValue, ConfigAuthority, WriterPaneSide, WriterPaneSidePolicy, resolve_field,
    };
    assert_eq!(
        recite_config::WriterConfig::default().pane_side,
        WriterPaneSide::Right
    );
    for authority in [
        ConfigAuthority::Project,
        ConfigAuthority::Generated,
        ConfigAuthority::Invocation,
    ] {
        assert!(
            resolve_field(
                WriterPaneSidePolicy,
                WriterPaneSide::Right,
                [AuthorityValue::new(authority, WriterPaneSide::Left)]
            )
            .is_err()
        );
    }
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("config.toml");
    let source = "config_version = 1\n[writer]\npane_side = 'above'\n";
    std::fs::write(&path, source)?;
    let store = UserConfigStore::new(resolve_config_path(
        Platform::Linux,
        &PlatformRoots::new(),
        Some(&path),
    )?);
    assert!(
        store
            .update(UserConfigEdit::WriterPaneSide(WriterPaneSide::Left))
            .is_err()
    );
    assert_eq!(std::fs::read_to_string(path)?, source);
    Ok(())
}
