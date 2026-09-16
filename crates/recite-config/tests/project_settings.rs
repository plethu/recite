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
