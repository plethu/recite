//! Checked, durable replacement shared by native Writer frontends.
use crate::FileError;
use atomic_write_file::AtomicWriteFile;
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

/// Preserve accepted source and the unapplied field draft before an explicit reload.
pub fn export_recovery(
    path: &Path,
    recovery: &crate::recovery::Recovery,
) -> Result<PathBuf, FileError> {
    let parent = path.parent().ok_or(FileError::Selection)?;
    let mut copy = tempfile::Builder::new()
        .prefix(".recite-recovered-")
        .suffix(".json")
        .tempfile_in(parent)?;
    serde_json::to_writer_pretty(&mut copy, recovery)?;
    copy.as_file().sync_all()?;
    let (_, path) = copy.keep().map_err(|error| error.error)?;
    sync_directory(parent)?;
    Ok(path)
}

pub fn read_regular(path: &Path) -> Result<String, FileError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() {
        return Err(FileError::FileKind);
    }
    Ok(fs::read_to_string(path)?)
}

/// Checked replacement shared by manuscript and standalone declaration sources.
pub fn replace_checked(path: &Path, baseline: &str, source: &str) -> Result<(), FileError> {
    replace_checked_with_sync(path, baseline, source, sync_directory)
}

fn replace_checked_with_sync(
    path: &Path,
    baseline: &str,
    source: &str,
    sync: impl FnOnce(&Path) -> io::Result<()>,
) -> Result<(), FileError> {
    let parent = path.parent().ok_or(FileError::Selection)?;
    let mut lock_name = path.as_os_str().to_owned();
    lock_name.push(".recite-editor.lock");
    let lock_path = PathBuf::from(lock_name);
    let lock_file = OpenOptions::new()
        .write(true)
        .read(true)
        .create(true)
        .truncate(false)
        .open(&lock_path)
        .map_err(FileError::Locked)?;
    lock_file
        .try_lock()
        .map_err(|error| FileError::Locked(io::Error::other(error)))?;
    let old = read_regular(path)?;
    // A prior replacement may be visible even though its directory sync failed.
    // Converge under the same lock without rewriting or creating another backup.
    if old == source {
        sync(parent)?;
        return Ok(());
    }
    if old != baseline {
        return Err(FileError::Conflict);
    }
    let permissions = fs::metadata(path)?.permissions();
    if permissions.readonly() {
        return Err(
            io::Error::new(io::ErrorKind::PermissionDenied, "Source file is read-only").into(),
        );
    }
    let mut replacement = AtomicWriteFile::open(path)?;
    replacement.as_file().set_permissions(permissions.clone())?;
    replacement.write_all(source.as_bytes())?;
    // Keep the last observed source even after successful replacement.
    let mut backup = tempfile::Builder::new()
        .prefix(".recite-editor-backup-")
        .tempfile_in(parent)?;
    backup.as_file().set_permissions(permissions)?;
    backup.write_all(old.as_bytes())?;
    backup.as_file().sync_all()?;
    if read_regular(path)? != baseline {
        return Err(FileError::Conflict);
    }
    backup.keep().map_err(|error| error.error)?;
    replacement.commit().map_err(FileError::Commit)?;
    sync(parent)?;
    Ok(())
}

pub fn sync_directory(parent: &Path) -> io::Result<()> {
    #[cfg(unix)]
    fs::File::open(parent)?.sync_all()?;
    #[cfg(not(unix))]
    let _ = parent;
    Ok(())
}

#[cfg(test)]
mod tests;
