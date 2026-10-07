use std::fs;

use recite_config::{ProjectSchemaError, discover_project};
use tempfile::TempDir;

type Result<T = ()> = std::result::Result<T, Box<dyn std::error::Error>>;

fn manifest(root: &std::path::Path, schema: Option<&str>) -> Result {
    let declaration = schema
        .map(|value| format!("[project]\nschema = {value:?}\n"))
        .unwrap_or_default();
    fs::write(
        root.join("recite.project.toml"),
        format!("format_version = 1\n{declaration}"),
    )?;
    Ok(())
}

#[test]
fn schema_loading_distinguishes_absent_invalid_and_missing_inputs() -> Result {
    let dir = TempDir::new()?;
    manifest(dir.path(), None)?;
    assert!(
        discover_project(dir.path())?
            .manifest()
            .load_schema()?
            .is_none()
    );
    for declaration in ["schema.json", "./schema.json"] {
        manifest(dir.path(), Some(declaration))?;
        let report = discover_project(dir.path())?;
        assert_eq!(
            report
                .manifest()
                .schema_key()?
                .ok_or("missing schema key")?
                .as_str(),
            "schema.json"
        );
        assert!(
            matches!(report.manifest().load_schema(), Err(ProjectSchemaError::Read { path, .. }) if path.ends_with("schema.json"))
        );
        fs::write(dir.path().join("schema.json"), "{ invalid")?;
        let loaded = report
            .manifest()
            .load_schema()?
            .ok_or("missing schema input")?;
        assert_eq!(loaded.key().as_str(), "schema.json");
        let loaded = loaded.into_report();
        assert!(loaded.schema.is_none());
        assert!(!loaded.diagnostics.is_empty());
        assert_eq!(
            loaded.diagnostics[0].span.file,
            dir.path()
                .join("schema.json")
                .to_string_lossy()
                .replace('\\', "/")
        );
        fs::write(dir.path().join("schema.json"), [0xff])?;
        assert!(
            matches!(report.manifest().load_schema(), Err(ProjectSchemaError::Read { source, .. }) if source.kind() == std::io::ErrorKind::InvalidData)
        );
        fs::remove_file(dir.path().join("schema.json"))?;
    }
    Ok(())
}

#[test]
fn schema_keys_reject_traversal_empty_and_outside_absolute_declarations() -> Result {
    let dir = TempDir::new()?;
    let outside = TempDir::new()?;
    for declaration in [
        "",
        ".",
        "../schema.json",
        "nested/../schema.json",
        outside
            .path()
            .join("schema.json")
            .to_str()
            .ok_or("non-UTF8 path")?,
    ] {
        manifest(dir.path(), Some(declaration))?;
        let report = discover_project(dir.path())?;
        assert!(
            matches!(
                report.manifest().schema_key(),
                Err(ProjectSchemaError::InvalidPath { .. })
            ),
            "accepted {declaration:?}"
        );
        assert!(matches!(
            report.manifest().load_schema(),
            Err(ProjectSchemaError::InvalidPath { .. })
        ));
    }
    Ok(())
}

#[cfg(unix)]
#[test]
fn schema_aliases_keep_declared_keys_but_must_resolve_inside_the_project() -> Result {
    use std::os::unix::fs::symlink;
    let dir = TempDir::new()?;
    let outside = TempDir::new()?;
    fs::write(dir.path().join("actual.json"), "{\"schema_version\":1}")?;
    symlink(dir.path().join("actual.json"), dir.path().join("link.json"))?;
    manifest(dir.path(), Some("link.json"))?;
    let loaded = discover_project(dir.path())?
        .manifest()
        .load_schema()?
        .ok_or("missing schema")?;
    assert_eq!(loaded.key().as_str(), "link.json");
    assert_eq!(
        loaded.path(),
        fs::canonicalize(dir.path().join("actual.json"))?
    );
    assert!(loaded.into_report().diagnostics.is_empty());
    fs::write(dir.path().join("actual.json"), "{ invalid")?;
    let invalid = discover_project(dir.path())?
        .manifest()
        .load_schema()?
        .ok_or("missing aliased schema")?
        .into_report();
    assert!(invalid.schema.is_none());
    assert_eq!(
        invalid.diagnostics[0].span.file,
        dir.path()
            .join("link.json")
            .to_string_lossy()
            .replace('\\', "/")
    );
    fs::remove_file(dir.path().join("link.json"))?;
    fs::write(
        outside.path().join("external.json"),
        "{\"schema_version\":1}",
    )?;
    symlink(
        outside.path().join("external.json"),
        dir.path().join("link.json"),
    )?;
    assert!(
        matches!(discover_project(dir.path())?.manifest().load_schema(), Err(ProjectSchemaError::OutsideProject { declared, resolved }) if declared.ends_with("link.json") && resolved.ends_with("external.json"))
    );
    Ok(())
}
