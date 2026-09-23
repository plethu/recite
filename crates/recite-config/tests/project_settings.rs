#[test]
fn project_settings_validate_and_refuse_stale_replacement() -> Result<(), Box<dyn std::error::Error>>
{
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("recite.project.toml");
    let original = "# Keep my comment
format_version = 1
[project]
content_set = 'original'
";
    std::fs::write(&path, original)?;
    let mut settings = recite_config::ProjectSettings::open(dir.path())?;
    assert!(
        settings
            .save(
                "[project
"
            )
            .is_err()
    );
    assert_eq!(std::fs::read_to_string(&path)?, original);
    assert!(
        settings
            .save(&original.replace("format_version = 1", "format_version = 999"))
            .is_err()
    );
    assert_eq!(std::fs::read_to_string(&path)?, original);
    assert!(
        settings
            .save(&format!(
                "{original}\n[discovery]\nsource_roots = ['missing']\n"
            ))
            .is_err()
    );
    assert_eq!(std::fs::read_to_string(&path)?, original);
    settings.save(&original.replace("original", "revised"))?;
    assert!(std::fs::read_to_string(&path)?.contains("# Keep my comment"));
    std::fs::write(&path, original)?;
    assert!(
        settings
            .save(&original.replace("original", "stale"))
            .is_err()
    );
    assert_eq!(std::fs::read_to_string(path)?, original);
    Ok(())
}

#[test]
fn manifest_edits_preserve_open_documents_before_writing() -> Result<(), Box<dyn std::error::Error>>
{
    let dir = tempfile::tempdir()?;
    let path = dir.path().join("recite.project.toml");
    let original = "format_version = 1\n[project]\ncontent_set = 'trial'\n";
    std::fs::write(&path, original)?;
    std::fs::write(dir.path().join("active.recite"), ":: start\n-> END\n")?;
    std::fs::write(dir.path().join("retained.recite"), ":: other\n-> END\n")?;
    let active = std::fs::canonicalize(dir.path().join("active.recite"))?;
    let retained = std::fs::canonicalize(dir.path().join("retained.recite"))?;
    let documents = [active.clone(), retained.clone()];
    let mut settings = recite_config::ProjectSettings::open(dir.path())?;
    for (name, removed) in [("active.recite", active), ("retained.recite", retained)] {
        let replacement = format!("{original}\n[discovery]\nexcludes = ['{name}']\n");
        assert!(matches!(
            settings.save_preserving_documents(&replacement, &documents),
            Err(recite_config::ProjectSettingsError::OpenDocument(path)) if path == removed
        ));
        assert_eq!(std::fs::read_to_string(&path)?, original);
        assert_eq!(settings.source(), original);
    }
    let valid = original.replace("trial", "revised");
    settings.save_preserving_documents(&valid, &documents)?;
    assert_eq!(std::fs::read_to_string(&path)?, valid);
    assert_eq!(settings.source(), valid);
    Ok(())
}
