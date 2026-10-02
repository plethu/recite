use super::SchemaIndex;
use std::fs;

const VALID: &str = r#"{"schema_version":1,"speakers":{"narrator":{"display_name":"Narrator"}}}"#;

#[test]
fn disk_refresh_matches_fresh_load_through_replacement_errors_and_recovery() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("schema.json");
    fs::write(&path, VALID).unwrap();
    let mut cached = SchemaIndex::load(Some(path.clone()));
    assert!(cached.schema().is_some());
    assert_eq!(cached.base(), cached);
    for text in [
        VALID.replace("Narrator", "Changed"),
        "{".into(),
        VALID.into(),
    ] {
        fs::write(&path, text).unwrap();
        cached = cached.base();
        assert_eq!(cached, SchemaIndex::load(Some(path.clone())));
        assert_eq!(cached.base(), cached);
    }
    fs::remove_file(&path).unwrap();
    cached = cached.base();
    assert!(cached.schema().is_none());
    assert!(!cached.diagnostics.is_empty());
    assert_eq!(cached, SchemaIndex::load(Some(path.clone())));
    fs::write(&path, VALID).unwrap();
    assert_eq!(cached.base(), SchemaIndex::load(Some(path)));
}

#[test]
fn closing_overlay_restores_fresh_disk_state_and_configured_uri() {
    let dir = tempfile::tempdir().unwrap();
    let path = dir.path().join("schema.json");
    fs::write(&path, VALID).unwrap();
    let disk = SchemaIndex::load(Some(path.clone()));
    let overlay = disk.overlay_for_open(disk.protocol_uri().unwrap(), "{", 8);
    assert!(overlay.schema().is_none());
    fs::write(&path, VALID.replace("Narrator", "Changed")).unwrap();
    assert_eq!(overlay.base(), SchemaIndex::load(Some(path.clone())));
    let same_text = disk.overlay_for_open(disk.protocol_uri().unwrap(), VALID, 9);
    fs::write(&path, VALID).unwrap();
    assert_eq!(same_text.base(), disk);
}

#[cfg(unix)]
#[test]
fn identical_invalid_bytes_at_retargeted_alias_use_new_diagnostic_path() {
    use std::os::unix::fs::symlink;
    let dir = tempfile::tempdir().unwrap();
    let first = dir.path().join("first.json");
    let second = dir.path().join("second.json");
    let alias = dir.path().join("schema.json");
    fs::write(&first, "{").unwrap();
    fs::write(&second, "{").unwrap();
    symlink(&first, &alias).unwrap();
    let cached = SchemaIndex::load(Some(alias.clone()));
    fs::remove_file(&alias).unwrap();
    symlink(&second, &alias).unwrap();
    let refreshed = cached.base();
    assert_ne!(refreshed.diagnostics, cached.diagnostics);
    assert_eq!(refreshed, SchemaIndex::load(Some(alias)));
}
