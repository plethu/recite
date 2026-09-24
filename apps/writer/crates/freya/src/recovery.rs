mod worker;
pub(crate) use worker::SnapshotStore;
pub(crate) type RecoveryStore = SnapshotStore<Recovery>;
// A locked, atomic recovery snapshot beside each edited document.
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
    pub baseline: std::sync::Arc<str>,
    pub draft: RecoveredDraft,
}

impl Recovery {
    pub fn new(baseline: std::sync::Arc<str>, draft: RecoveredDraft) -> Self {
        Self {
            version: 1,
            baseline,
            draft,
        }
    }
}

pub(crate) trait Snapshot:
    Clone + PartialEq + Serialize + serde::de::DeserializeOwned + Send + 'static
{
    fn valid_version(&self) -> bool;
}
impl Snapshot for Recovery {
    fn valid_version(&self) -> bool {
        self.version == 1
    }
}

struct RecoveryDisk<T: Snapshot> {
    path: PathBuf,
    // OS lock is released even after a process crash; the empty lock file remains.
    lock: File,
    persisted: Option<T>,
}

impl<T: Snapshot> Drop for RecoveryDisk<T> {
    fn drop(&mut self) {
        // Release ownership even if a concurrently spawned child still holds a
        // duplicate descriptor before exec. Closing our descriptor alone waits
        // for every duplicate to close. A process crash still releases the lock.
        let _ = self.lock.unlock();
    }
}

impl<T: Snapshot> RecoveryDisk<T> {
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
                let recovery: T = serde_json::from_str(&text)?;
                if !recovery.valid_version() {
                    return Err(FileError::RecoveryVersion);
                }
                Some(recovery)
            }
            Err(FileError::Io(error)) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(error),
        };
        Ok(Self {
            path,
            lock,
            persisted,
        })
    }

    pub fn snapshot(&self) -> Option<&T> {
        self.persisted.as_ref()
    }

    pub fn persist(&mut self, recovery: Option<T>) -> Result<(), FileError> {
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

pub(crate) mod status;
