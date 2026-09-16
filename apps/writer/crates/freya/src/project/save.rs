use super::{FileError, ProjectFiles, read_regular};
use atomic_write_file::AtomicWriteFile;
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::PathBuf,
};

impl ProjectFiles {
    /// Cooperative lock + checked replacement. Each replacement retains a backup
    /// of the previous bytes. Non-cooperating writers can still race the final check.
    pub fn save(&mut self, source: &str) -> Result<(), FileError> {
        let parent = self.current.parent().ok_or(FileError::Selection)?;
        let mut lock_name = self.current.as_os_str().to_owned();
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
        let old = read_regular(&self.current)?;
        if old != self.saved {
            return Err(FileError::Conflict);
        }
        if source == self.saved {
            #[cfg(unix)]
            fs::File::open(parent)?.sync_all()?;
            return Ok(());
        }
        let permissions = fs::metadata(&self.current)?.permissions();
        if permissions.readonly() {
            return Err(io::Error::new(
                io::ErrorKind::PermissionDenied,
                "Source file is read-only",
            )
            .into());
        }
        let mut replacement = AtomicWriteFile::open(&self.current)?;
        replacement.as_file().set_permissions(permissions.clone())?;
        replacement.write_all(source.as_bytes())?;
        // Keep the last observed source even after successful replacement.
        let mut backup = tempfile::Builder::new()
            .prefix(".recite-editor-backup-")
            .tempfile_in(parent)?;
        backup.as_file().set_permissions(permissions)?;
        backup.write_all(old.as_bytes())?;
        backup.as_file().sync_all()?;
        if read_regular(&self.current)? != self.saved {
            return Err(FileError::Conflict);
        }
        backup.keep().map_err(|error| error.error)?;
        replacement.commit().map_err(FileError::Commit)?;
        self.saved = source.to_owned();
        #[cfg(unix)]
        fs::File::open(parent)?.sync_all()?;
        Ok(())
    }
}
