//! File ownership for the first retained editor slice.
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

#[derive(Debug, thiserror::Error)]
pub enum FileError {
    #[error(transparent)]
    Discovery(#[from] recite_config::ProjectDiscoveryError),
    #[error(
        "Project discovery is incomplete. Repair the project's discovery diagnostics before opening it here."
    )]
    Incomplete,
    #[error("The project has no Recite source files.")]
    Empty,
    #[error("The selected file is no longer in this project.")]
    Selection,
    #[error(
        "This file changed on disk. Your edits remain open; save them elsewhere before reopening the project."
    )]
    Conflict,
    #[error("Refusing to replace a symbolic link or a non-regular file.")]
    FileKind,
    #[error(
        "Could not acquire the editor's save lock: {0}. A crashed editor may have left a .recite-editor.lock file beside the source."
    )]
    Locked(io::Error),
    #[error("File operation failed: {0}")]
    Io(#[from] io::Error),
}

pub struct ProjectFiles {
    pub paths: Vec<PathBuf>,
    pub current: PathBuf,
    saved: String,
    names: std::collections::BTreeMap<PathBuf, String>,
}

impl ProjectFiles {
    pub fn open(path: &Path) -> Result<Self, FileError> {
        let report = recite_config::discover_project(path)?;
        if !report.is_complete() {
            return Err(FileError::Incomplete);
        }
        let paths: Vec<_> = report
            .documents()
            .iter()
            .map(|d| d.path().to_owned())
            .collect();
        let current = paths.first().ok_or(FileError::Empty)?.clone();
        let saved = read_regular(&current)?;
        let names = report
            .documents()
            .iter()
            .map(|d| (d.path().to_owned(), d.key().as_str().to_owned()))
            .collect();
        Ok(Self {
            paths,
            current,
            saved,
            names,
        })
    }

    pub fn document_name(&self) -> Result<&str, FileError> {
        self.names
            .get(&self.current)
            .map(String::as_str)
            .ok_or(FileError::Selection)
    }

    pub fn source(&self) -> &str {
        &self.saved
    }

    pub fn dirty(&self, source: &str) -> bool {
        self.saved != source
    }

    pub fn select(&mut self, path: &Path) -> Result<(), FileError> {
        if !self.paths.iter().any(|p| p == path) {
            return Err(FileError::Selection);
        }
        let source = read_regular(path)?;
        self.current = path.to_owned();
        self.saved = source;
        Ok(())
    }

    /// Cooperative lock + checked replacement. Each replacement retains a backup
    /// of the previous bytes. Non-cooperating writers can still race the final check.
    pub fn save(&mut self, source: &str) -> Result<(), FileError> {
        let parent = self.current.parent().ok_or(FileError::Selection)?;
        let mut lock_name = self.current.as_os_str().to_owned();
        lock_name.push(".recite-editor.lock");
        let lock_path = PathBuf::from(lock_name);
        let lock_file = OpenOptions::new()
            .write(true)
            .create_new(true)
            .open(&lock_path)
            .map_err(FileError::Locked)?;
        let _lock = SaveLock {
            path: lock_path,
            _file: lock_file,
        };
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
        let mut replacement = tempfile::NamedTempFile::new_in(parent)?;
        replacement.as_file().set_permissions(permissions.clone())?;
        replacement.write_all(source.as_bytes())?;
        replacement.as_file().sync_all()?;
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
        replacement
            .persist(&self.current)
            .map_err(|error| error.error)?;
        self.saved = source.to_owned();
        #[cfg(unix)]
        fs::File::open(parent)?.sync_all()?;
        Ok(())
    }
}

fn read_regular(path: &Path) -> Result<String, FileError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() {
        return Err(FileError::FileKind);
    }
    Ok(fs::read_to_string(path)?)
}

struct SaveLock {
    path: PathBuf,
    _file: fs::File,
}
impl Drop for SaveLock {
    fn drop(&mut self) {
        let _ = fs::remove_file(&self.path);
    }
}

#[cfg(test)]
mod tests;
