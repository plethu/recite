use std::{fs, io};

use super::{FileError, ManifestDraft};

fn draft() -> Result<(tempfile::TempDir, ManifestDraft), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    fs::write(
        root.path().join("recite.project.toml"),
        "format_version = 1\n",
    )?;
    let mut draft = ManifestDraft::open(root.path())?;
    draft.set(
        "format_version = 1\n# edited\n".to_owned(),
        Default::default(),
        vec!["scene.recite".to_owned()],
    )?;
    Ok((root, draft))
}

#[test]
fn failed_recovery_removal_remains_saveable_after_manifest_commit()
-> Result<(), Box<dyn std::error::Error>> {
    let (root, mut draft) = draft()?;
    fs::remove_file(&draft.recovery)?;
    fs::create_dir(&draft.recovery)?;
    assert!(matches!(draft.save(), Err(FileError::Io(_))));
    assert_eq!(fs::read_to_string(&draft.path)?, draft.text());
    assert!(
        draft.dirty(),
        "cleanup failure must remain visible to save_all"
    );
    let files = fs::read_dir(root.path())?.count();
    fs::remove_dir(&draft.recovery)?;
    draft.save()?;
    assert!(!draft.dirty());
    assert!(!draft.recovery.exists());
    assert_eq!(
        fs::read_dir(root.path())?.count(),
        files - 1,
        "retry must not create another backup"
    );
    drop(draft);
    assert!(!ManifestDraft::open(root.path())?.dirty());
    Ok(())
}

#[test]
fn cleanup_sync_failure_remains_pending_after_recovery_file_disappears()
-> Result<(), Box<dyn std::error::Error>> {
    let (_root, mut draft) = draft()?;
    let result = draft.save_with_cleanup_sync(|_| Err(io::Error::other("cleanup sync failed")));
    assert!(matches!(result, Err(FileError::Io(_))));
    assert!(!draft.recovery.exists());
    assert!(draft.dirty());
    draft.save()?;
    assert!(!draft.dirty());
    Ok(())
}

#[test]
fn reopen_reconciles_a_committed_manifest_and_old_recovery_baseline()
-> Result<(), Box<dyn std::error::Error>> {
    let (root, draft) = draft()?;
    fs::write(&draft.path, draft.text())?;
    drop(draft);
    let mut recovered = ManifestDraft::open(root.path())?;
    assert!(recovered.dirty());
    recovered.save()?;
    assert!(!recovered.dirty());
    assert!(!recovered.recovery.exists());
    Ok(())
}

#[test]
fn recovery_cleanup_does_not_discard_a_draft_when_disk_changed_again()
-> Result<(), Box<dyn std::error::Error>> {
    let (_root, mut draft) = draft()?;
    let recovery = fs::read(&draft.recovery)?;
    assert!(matches!(
        draft.save_with_cleanup_sync(|_| Err(io::Error::other("cleanup sync failed"))),
        Err(FileError::Io(_))
    ));
    fs::write(&draft.recovery, &recovery)?;
    fs::write(&draft.path, "format_version = 1\n# external\n")?;
    assert!(matches!(draft.save(), Err(FileError::Conflict)));
    assert!(draft.dirty());
    assert_eq!(fs::read(&draft.recovery)?, recovery);
    assert_eq!(draft.text(), "format_version = 1\n# edited\n");
    Ok(())
}
