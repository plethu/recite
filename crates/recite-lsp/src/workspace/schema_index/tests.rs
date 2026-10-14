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

#[test]
fn clones_share_immutable_schema_payload_and_disk_changes_replace_it()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("schema.toml");
    std::fs::write(
        &path,
        "schema_version = 1\n[producer]\nid = \"schema-test\"\n[conditions.ready]\nparams = []\nreturns = \"bool\"\n",
    )?;
    let original = super::SchemaIndex::load(Some(path.clone()));
    assert_eq!(original, super::SchemaIndex::load(Some(path.clone())));
    let clone = original.clone();
    let unchanged = original.base();
    assert!(std::sync::Arc::ptr_eq(
        original.schema.as_ref().ok_or("schema")?,
        clone.schema.as_ref().ok_or("clone")?
    ));
    assert!(std::sync::Arc::ptr_eq(
        original.summary.as_ref().ok_or("summary")?,
        clone.summary.as_ref().ok_or("clone summary")?
    ));
    assert!(std::sync::Arc::ptr_eq(
        original.source.as_ref().ok_or("source")?,
        clone.source.as_ref().ok_or("clone source")?
    ));
    assert!(std::sync::Arc::ptr_eq(
        original.source.as_ref().ok_or("source")?,
        unchanged.source.as_ref().ok_or("unchanged source")?
    ));
    assert!(original.code_action_document().is_none());
    std::fs::write(
        &path,
        "schema_version = 1\n[producer]\nid = \"schema-test\"\n[conditions.changed]\nparams = []\nreturns = \"bool\"\n",
    )?;
    let changed = original.base();
    assert!(!std::sync::Arc::ptr_eq(
        original.schema.as_ref().ok_or("schema")?,
        changed.schema.as_ref().ok_or("changed schema")?
    ));
    assert!(
        original
            .summary()
            .ok_or("original")?
            .conditions()
            .iter()
            .any(|c| c.name() == "ready")
    );
    assert!(
        changed
            .summary()
            .ok_or("changed")?
            .conditions()
            .iter()
            .any(|c| c.name() == "changed")
    );
    Ok(())
}

#[test]
fn overlay_replacement_and_owned_code_action_edits_preserve_old_payloads()
-> Result<(), Box<dyn std::error::Error>> {
    use recite_core::schema::SchemaSourceEdit;
    let directory = tempfile::tempdir()?;
    let path = directory.path().join("schema.toml");
    let text = "schema_version = 1\n[producer]\nid = \"schema-test\"\n[speakers.narrator]\ndisplay_name = \"Before\"\n";
    std::fs::write(&path, text)?;
    let saved = SchemaIndex::load(Some(path));
    let uri = saved.protocol_uri().ok_or("uri")?;
    let overlay = saved.overlay_for_open(uri.clone(), text, 1);
    let before = overlay.clone();
    let mut editable = overlay.code_action_document().ok_or("editable TOML")?;
    editable
        .source
        .apply_edit(SchemaSourceEdit::SetSpeakerDisplayName {
            name: "narrator".into(),
            display_name: Some("Edited".into()),
        })?;
    assert_eq!(overlay, before);
    assert_eq!(overlay.source.as_ref().ok_or("source")?.source_text(), text);
    assert!(editable.source.source_text().contains("Edited"));
    let replacement = overlay.overlay_for_open(uri, "{", 2);
    assert!(replacement.schema().is_none());
    assert!(!replacement.diagnostics.is_empty());
    assert_eq!(overlay.text.as_deref(), Some(text));
    assert!(overlay.diagnostics.is_empty());
    assert_eq!(overlay, before);
    Ok(())
}
