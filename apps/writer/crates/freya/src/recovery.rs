//! A locked, atomic recovery snapshot beside each edited document.
use std::{
    fs::{self, File, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

use recite_writer_model::RecoveredDraft;
use serde::{Deserialize, Serialize};

use crate::project::{FileError, read_regular};

#[derive(Clone, Debug, PartialEq, Serialize, Deserialize)]
pub(super) struct Recovery {
    version: u32,
    pub baseline: String,
    pub draft: RecoveredDraft,
}

impl Recovery {
    pub fn new(baseline: String, draft: RecoveredDraft) -> Self {
        Self {
            version: 1,
            baseline,
            draft,
        }
    }
}

pub(super) struct RecoveryStore {
    path: PathBuf,
    // OS lock is released even after a process crash; the empty lock file remains.
    _lock: File,
    persisted: Option<Recovery>,
}

impl RecoveryStore {
    pub fn open(source: &Path) -> Result<Self, FileError> {
        let path = sidecar(source, ".recite-editor-recovery.json");
        let lock_path = sidecar(source, ".recite-editor-recovery.lock");
        match fs::symlink_metadata(&lock_path) {
            Ok(metadata) if !metadata.is_file() => return Err(FileError::FileKind),
            Ok(_) => {}
            Err(error) if error.kind() == io::ErrorKind::NotFound => {}
            Err(error) => return Err(error.into()),
        }
        let lock = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(lock_path)?;
        lock.try_lock().map_err(|error| match error {
            fs::TryLockError::WouldBlock => FileError::RecoveryInUse,
            fs::TryLockError::Error(error) => FileError::Io(error),
        })?;
        let persisted = match read_regular(&path) {
            Ok(text) => {
                let recovery: Recovery = serde_json::from_str(&text)?;
                if recovery.version != 1 {
                    return Err(FileError::RecoveryVersion);
                }
                Some(recovery)
            }
            Err(FileError::Io(error)) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(error),
        };
        Ok(Self {
            path,
            _lock: lock,
            persisted,
        })
    }

    pub fn snapshot(&self) -> Option<&Recovery> {
        self.persisted.as_ref()
    }

    pub fn persist(&mut self, recovery: Option<Recovery>) -> Result<(), FileError> {
        if recovery == self.persisted {
            return Ok(());
        }
        let parent = self.path.parent().ok_or(FileError::Selection)?;
        if let Some(recovery) = &recovery {
            let mut temporary = tempfile::NamedTempFile::new_in(parent)?;
            serde_json::to_writer(&mut temporary, recovery)?;
            temporary.flush()?;
            temporary.as_file().sync_all()?;
            temporary.persist(&self.path).map_err(|error| error.error)?;
        } else {
            match fs::remove_file(&self.path) {
                Ok(()) => {}
                Err(error) if error.kind() == io::ErrorKind::NotFound => {}
                Err(error) => return Err(error.into()),
            }
        }
        #[cfg(unix)]
        File::open(parent)?.sync_all()?;
        self.persisted = recovery;
        Ok(())
    }
}

fn sidecar(source: &Path, suffix: &str) -> PathBuf {
    let mut path = source.as_os_str().to_owned();
    path.push(suffix);
    path.into()
}

#[cfg(test)]
mod tests;
