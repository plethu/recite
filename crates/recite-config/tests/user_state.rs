use recite_config::UserStateFile;

#[test]
fn state_updates_reload_before_merging_and_reject_invalid_changes()
-> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("state.json");
    let first = UserStateFile::new(path.clone());
    let second = UserStateFile::new(path.clone());
    first.update(|_| Ok::<_, std::io::Error>("one".into()))?;
    second.update(|previous| {
        Ok::<_, std::io::Error>(format!("{} two", previous.unwrap_or_default()))
    })?;
    assert_eq!(first.load()?.as_deref(), Some("one two"));
    assert!(
        first
            .update(|_| Err::<String, _>(std::io::Error::other("reject")))
            .is_err()
    );
    assert_eq!(std::fs::read_to_string(path)?, "one two");
    Ok(())
}
#[cfg(unix)]
#[test]
fn application_state_does_not_follow_a_symlink() -> Result<(), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    let target = dir.path().join("target");
    let path = dir.path().join("state");
    std::fs::write(&target, "untouched")?;
    std::os::unix::fs::symlink(&target, &path)?;
    let state = UserStateFile::new(path);
    assert!(state.load().is_err());
    assert!(
        state
            .update(|_| Ok::<_, std::io::Error>("replacement".into()))
            .is_err()
    );
    assert_eq!(std::fs::read_to_string(target)?, "untouched");
    Ok(())
}

#[test]
fn state_names_cannot_escape_the_user_directory_or_replace_preferences()
-> Result<(), Box<dyn std::error::Error>> {
    use recite_config::{Platform, PlatformRoots, UserConfigStore, resolve_config_path};
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("config.toml");
    let store = UserConfigStore::new(resolve_config_path(
        Platform::Linux,
        &PlatformRoots::new(),
        Some(&path),
    )?);
    for name in [
        "",
        "..",
        "../scene.json",
        "a/b",
        "a\\b",
        "config.toml",
        "CONFIG.TOML",
    ] {
        assert!(store.state_file(name).is_err(), "{name}");
    }
    store
        .state_file("writer-layouts.json")?
        .update(|_| Ok::<_, std::io::Error>("{}".into()))?;
    assert!(!path.exists());
    assert_eq!(
        std::fs::read_to_string(directory.path().join("writer-layouts.json"))?,
        "{}"
    );
    Ok(())
}
