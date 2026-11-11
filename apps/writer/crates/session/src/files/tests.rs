use super::*;
#[cfg(unix)]
use std::os::unix::fs as unix_fs;

#[test]
fn retry_after_visible_replacement_confirms_sync_without_another_backup()
-> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let path = root.path().join("scene.recite");
    fs::write(&path, "before")?;
    let error = replace_checked_with_sync(&path, "before", "after", |_| {
        Err(io::Error::other("directory sync failed"))
    });
    assert!(matches!(error, Err(FileError::Io(_))));
    assert_eq!(fs::read_to_string(&path)?, "after");
    let files_before_retry = fs::read_dir(root.path())?.count();
    replace_checked(&path, "before", "after")?;
    assert_eq!(fs::read_dir(root.path())?.count(), files_before_retry);
    fs::write(&path, "someone else's edit")?;
    assert!(matches!(
        replace_checked(&path, "before", "after"),
        Err(FileError::Conflict)
    ));
    assert_eq!(fs::read_to_string(path)?, "someone else's edit");
    Ok(())
}

#[test]
fn read_only_target_refuses_replacement_without_an_unchecked_fallback()
-> Result<(), Box<dyn std::error::Error>> {
    // A checkout tool or lock-based VCS can leave a file read-only; the atomic rename could
    // still replace it, so the permission is checked rather than left to the filesystem.
    let root = tempfile::tempdir()?;
    let path = root.path().join("scene.recite");
    fs::write(&path, "before")?;
    let mut permissions = fs::metadata(&path)?.permissions();
    permissions.set_readonly(true);
    fs::set_permissions(&path, permissions)?;
    let files_before = fs::read_dir(root.path())?
        .map(|entry| entry.map(|entry| entry.file_name()))
        .collect::<Result<Vec<_>, _>>()?;
    let refused = replace_checked(&path, "before", "after");
    assert!(
        matches!(&refused, Err(FileError::Io(error)) if error.kind() == io::ErrorKind::PermissionDenied),
        "{refused:?}"
    );
    assert_eq!(fs::read_to_string(&path)?, "before");
    assert!(fs::metadata(&path)?.permissions().readonly());
    let files_after = fs::read_dir(root.path())?
        .map(|entry| entry.map(|entry| entry.file_name()))
        .collect::<Result<Vec<_>, _>>()?;
    let unexpected = files_after
        .iter()
        .filter(|name| !files_before.contains(name) && !name.to_string_lossy().ends_with(".lock"))
        .collect::<Vec<_>>();
    assert!(unexpected.is_empty(), "refusal left {unexpected:?}");
    Ok(())
}

#[cfg(unix)]
#[test]
fn desired_content_does_not_bypass_symlink_rejection() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let target = root.path().join("target.recite");
    let link = root.path().join("link.recite");
    fs::write(&target, "after")?;
    unix_fs::symlink(&target, &link)?;
    assert!(matches!(
        replace_checked(&link, "before", "after"),
        Err(FileError::FileKind)
    ));
    Ok(())
}
