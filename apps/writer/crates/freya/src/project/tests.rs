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
        recite_writer_model::FIXTURE,
    )?;
    let files = ProjectFiles::open(dir.path())?;
    Ok((dir, files))
}

#[test]
fn save_preserves_source_and_keeps_previous_bytes() -> Result<(), Box<dyn std::error::Error>> {
    let (_dir, mut files) = project()?;
    let old = files.saved.clone();
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
    let old = files.saved.clone();
    let lock = files.current.with_file_name(format!(
        "{}.recite-editor.lock",
        files.current.file_name().ok_or("name")?.to_string_lossy()
    ));
    let held = fs::File::options()
        .create(true)
        .truncate(false)
        .read(true)
        .write(true)
        .open(&lock)?;
    held.try_lock()?;
    assert!(matches!(
        files.save("changed"),
        Err(super::FileError::Locked(_))
    ));
    assert_eq!(fs::read_to_string(&files.current)?, old);
    drop(held);
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

#[test]
fn restored_session_refuses_to_overwrite_changes_made_while_closed()
-> Result<(), Box<dyn std::error::Error>> {
    let (dir, mut files) = project()?;
    let mut workbench = files.workbench()?;
    workbench.set_draft("A recovered question.".into());
    workbench.apply()?;
    workbench.set_draft("Unaccepted field draft.".into());
    files.checkpoint(&workbench)?;
    let current = files.current.clone();
    drop(files);
    fs::write(&current, "external edit")?;
    let mut files = ProjectFiles::open(dir.path())?;
    let restored = files.workbench()?;
    assert!(
        restored
            .document()
            .source()
            .contains("A recovered question.")
    );
    assert_eq!(restored.draft(), "Unaccepted field draft.");
    assert!(matches!(
        files.save(restored.document().source()),
        Err(super::FileError::Conflict)
    ));
    let (copy, loaded) = files.reload(&restored)?;
    assert_eq!(loaded.document().source(), "external edit");
    let preserved: crate::recovery::Recovery = serde_json::from_str(&fs::read_to_string(copy)?)?;
    assert_eq!(preserved.draft, restored.recovery());
    assert_eq!(fs::read_to_string(current)?, "external edit");
    assert!(!files.has_recovery());
    Ok(())
}

#[test]
fn failed_file_switch_keeps_the_current_session_and_its_recovery()
-> Result<(), Box<dyn std::error::Error>> {
    let (dir, mut files) = project()?;
    let other = dir.path().join("second.recite");
    fs::write(&other, ":: second\n-> END\n")?;
    fs::write(
        dir.path().join("second.recite.recite-editor-recovery.json"),
        "broken",
    )?;
    let original = files.current.clone();
    assert!(files.select(&other).is_err());
    assert_eq!(files.current, original);
    assert_eq!(
        files.workbench()?.document().source(),
        recite_writer_model::FIXTURE
    );
    Ok(())
}
