use std::{fs, io};

use super::{FileError, replace_checked, replace_checked_with_sync};

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

#[cfg(unix)]
#[test]
fn desired_content_does_not_bypass_symlink_rejection() -> Result<(), Box<dyn std::error::Error>> {
    let root = tempfile::tempdir()?;
    let target = root.path().join("target.recite");
    let link = root.path().join("link.recite");
    fs::write(&target, "after")?;
    std::os::unix::fs::symlink(&target, &link)?;
    assert!(matches!(
        replace_checked(&link, "before", "after"),
        Err(FileError::FileKind)
    ));
    Ok(())
}
