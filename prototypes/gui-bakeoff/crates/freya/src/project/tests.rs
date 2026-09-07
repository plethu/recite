use super::ProjectFiles;
use std::fs;

fn project() -> Result<(tempfile::TempDir, ProjectFiles), Box<dyn std::error::Error>> {
    let dir = tempfile::tempdir()?;
    fs::write(
        dir.path().join("recite.project.toml"),
        "format_version = 1\n[project]\ncontent_set = \"trial\"\nversion = \"1\"\n",
    )?;
    fs::write(
        dir.path().join("scene.recite"),
        recite_bakeoff_authoring::FIXTURE,
    )?;
    let files = ProjectFiles::open(dir.path())?;
    Ok((dir, files))
}

#[test]
fn save_preserves_source_and_keeps_previous_bytes() -> Result<(), Box<dyn std::error::Error>> {
    let (_dir, mut files) = project()?;
    let old = files.source().to_owned();
    let changed = old.replace("Would you tell me", "Could you tell me");
    files.save(&changed)?;
    assert_eq!(fs::read_to_string(&files.current)?, changed);
    assert!(!files.dirty(&changed));
    let backups: Vec<_> = fs::read_dir(files.current.parent().ok_or("parent")?)?
        .collect::<Result<Vec<_>, _>>()?
        .into_iter()
        .filter(|p| {
            p.file_name()
                .to_string_lossy()
                .starts_with(".recite-editor-backup-")
        })
        .collect();
    assert_eq!(backups.len(), 1);
    assert_eq!(fs::read_to_string(backups[0].path())?, old);
    Ok(())
}

#[test]
fn external_changes_are_not_overwritten() -> Result<(), Box<dyn std::error::Error>> {
    let (_dir, mut files) = project()?;
    fs::write(&files.current, "external writer")?;
    assert!(matches!(
        files.save("my draft"),
        Err(super::FileError::Conflict)
    ));
    assert_eq!(fs::read_to_string(&files.current)?, "external writer");
    assert!(files.dirty("my draft"));
    Ok(())
}

#[test]
fn held_save_lock_preserves_the_file_and_retry_succeeds() -> Result<(), Box<dyn std::error::Error>>
{
    let (_dir, mut files) = project()?;
    let old = files.source().to_owned();
    let lock = files.current.with_file_name(format!(
        "{}.recite-editor.lock",
        files.current.file_name().ok_or("name")?.to_string_lossy()
    ));
    fs::write(&lock, "")?;
    assert!(matches!(
        files.save("changed"),
        Err(super::FileError::Locked(_))
    ));
    assert_eq!(fs::read_to_string(&files.current)?, old);
    fs::remove_file(lock)?;
    files.save("changed")?;
    assert_eq!(fs::read_to_string(&files.current)?, "changed");
    Ok(())
}

#[cfg(unix)]
#[test]
fn substituted_symlink_is_not_followed() -> Result<(), Box<dyn std::error::Error>> {
    let (dir, mut files) = project()?;
    let outside = dir.path().join("unrelated");
    fs::write(&outside, "untouched")?;
    fs::remove_file(&files.current)?;
    std::os::unix::fs::symlink(&outside, &files.current)?;
    assert!(matches!(
        files.save("changed"),
        Err(super::FileError::FileKind)
    ));
    assert_eq!(fs::read_to_string(outside)?, "untouched");
    Ok(())
}
