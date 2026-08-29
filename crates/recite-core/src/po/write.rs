use super::{PoDocument, PoDocumentFingerprint, PoParseError};
use std::fs::{self, File, OpenOptions};
use std::io::{self, Write};
use std::path::{Path, PathBuf};

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum PoIoError {
    #[error("failed to read PO document {path}: {source}")]
    Read { path: PathBuf, source: io::Error },
    #[error("failed to decode PO document {path} as UTF-8: {source}")]
    Utf8 {
        path: PathBuf,
        source: std::string::FromUtf8Error,
    },
    #[error("failed to parse PO document {path}: {source}")]
    Parse { path: PathBuf, source: PoParseError },
}

#[non_exhaustive]
#[derive(Debug, thiserror::Error)]
pub enum PoWriteError {
    #[error("refusing to write through symlink PO document {path}")]
    Symlink { path: PathBuf },
    #[error("could not acquire cooperative PO writer lock beside {path}: {source}")]
    Lock { path: PathBuf, source: io::Error },
    #[error("failed to read PO document {path} before writing: {source}")]
    Read { path: PathBuf, source: io::Error },
    #[error(
        "PO document changed on disk before writing {path} (expected {expected}, found {actual})"
    )]
    Conflict {
        path: PathBuf,
        expected: PoDocumentFingerprint,
        actual: PoDocumentFingerprint,
    },
    #[error("failed to create temporary PO document beside {path}: {source}")]
    CreateTemp { path: PathBuf, source: io::Error },
    #[error("failed to write temporary PO document beside {path}: {source}")]
    Write { path: PathBuf, source: io::Error },
    #[error("failed to atomically replace PO document {path}: {source}")]
    Replace { path: PathBuf, source: io::Error },
}

impl PoDocument {
    /// Load and parse a PO file while retaining its path in diagnostics.
    pub fn read(path: impl AsRef<Path>) -> Result<Self, PoIoError> {
        let path = path.as_ref().to_owned();
        let bytes = fs::read(&path).map_err(|source| PoIoError::Read {
            path: path.clone(),
            source,
        })?;
        let source = String::from_utf8(bytes).map_err(|source| PoIoError::Utf8 {
            path: path.clone(),
            source,
        })?;
        Self::parse_with_path(path.display().to_string(), source)
            .map_err(|source| PoIoError::Parse { path, source })
    }

    /// Alias for [`PoDocument::read`].
    pub fn load(path: impl AsRef<Path>) -> Result<Self, PoIoError> {
        Self::read(path)
    }

    /// Atomically replace a PO file after a fingerprint check.
    ///
    /// Recite writers serialize through an OS-locked same-directory sidecar
    /// file and re-read the target immediately before replacement. The sidecar
    /// remains after the write, while its OS lock is released with the handle.
    /// An editor that does not cooperate with that lock can still write after
    /// the final read; no portable API can close that residual race. The
    /// replacement primitive syncs the temporary file and best-effort syncs
    /// its parent directory on Unix. File mode is
    /// retained, but ownership, ACLs, xattrs, and other extended metadata are
    /// not promised to survive replacement.
    pub fn write_atomically(
        &self,
        path: impl AsRef<Path>,
        expected: &PoDocumentFingerprint,
    ) -> Result<PoDocumentFingerprint, PoWriteError> {
        let path = path.as_ref().to_owned();
        let _lock = WriterLock::acquire(&path)?;
        if fs::symlink_metadata(&path)
            .map_err(|source| PoWriteError::Read {
                path: path.clone(),
                source,
            })?
            .file_type()
            .is_symlink()
        {
            return Err(PoWriteError::Symlink { path });
        }
        let current = fs::read(&path).map_err(|source| PoWriteError::Read {
            path: path.clone(),
            source,
        })?;
        let permissions = fs::metadata(&path)
            .map_err(|source| PoWriteError::Read {
                path: path.clone(),
                source,
            })?
            .permissions();
        let actual = PoDocumentFingerprint::from_bytes(&current);
        if &actual != expected {
            return Err(PoWriteError::Conflict {
                path,
                expected: expected.clone(),
                actual,
            });
        }

        let parent = normalized_parent(&path);
        let mut temporary = atomic_write_file::AtomicWriteFile::options()
            .open(&path)
            .map_err(|source| PoWriteError::CreateTemp {
                path: path.clone(),
                source,
            })?;
        temporary
            .as_file_mut()
            .set_permissions(permissions)
            .map_err(|source| PoWriteError::Write {
                path: path.clone(),
                source,
            })?;
        write_and_sync(temporary.as_file_mut(), self.source.as_bytes()).map_err(|source| {
            PoWriteError::Write {
                path: path.clone(),
                source,
            }
        })?;
        // Check again after preparing the replacement while the cooperative
        // writer lock remains held.
        let latest = match fs::read(&path) {
            Ok(latest) => latest,
            Err(source) => {
                return Err(PoWriteError::Read { path, source });
            }
        };
        let actual = PoDocumentFingerprint::from_bytes(&latest);
        if &actual != expected {
            return Err(PoWriteError::Conflict {
                path,
                expected: expected.clone(),
                actual,
            });
        }
        temporary.commit().map_err(|source| PoWriteError::Replace {
            path: path.clone(),
            source,
        })?;
        #[cfg(unix)]
        sync_parent(parent);
        Ok(self.fingerprint())
    }

    /// Alias for [`PoDocument::write_atomically`].
    pub fn save(
        &self,
        path: impl AsRef<Path>,
        expected: &PoDocumentFingerprint,
    ) -> Result<PoDocumentFingerprint, PoWriteError> {
        self.write_atomically(path, expected)
    }

    /// Alias for [`PoDocument::write_atomically`].
    pub fn write_atomic(
        &self,
        path: impl AsRef<Path>,
        expected: &PoDocumentFingerprint,
    ) -> Result<PoDocumentFingerprint, PoWriteError> {
        self.write_atomically(path, expected)
    }
}

fn write_and_sync(file: &mut File, bytes: &[u8]) -> io::Result<()> {
    file.write_all(bytes)?;
    file.flush()?;
    file.sync_all()
}

struct WriterLock {
    file: File,
}

impl WriterLock {
    fn acquire(target: &Path) -> Result<Self, PoWriteError> {
        let path = lock_path(target);
        let file = OpenOptions::new()
            .read(true)
            .write(true)
            .create(true)
            .truncate(false)
            .open(&path)
            .map_err(|source| PoWriteError::Lock {
                path: path.clone(),
                source,
            })?;
        file.lock().map_err(|source| PoWriteError::Lock {
            path: path.clone(),
            source,
        })?;
        Ok(Self { file })
    }
}

impl Drop for WriterLock {
    fn drop(&mut self) {
        let _ = self.file.unlock();
    }
}

fn lock_path(target: &Path) -> PathBuf {
    let parent = normalized_parent(target);
    let name = target
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("catalog.po");
    parent.join(format!(".{name}.recite.lock"))
}

fn normalized_parent(path: &Path) -> &Path {
    path.parent()
        .filter(|parent| !parent.as_os_str().is_empty())
        .unwrap_or_else(|| Path::new("."))
}

#[cfg(unix)]
fn sync_parent(parent: &Path) {
    if let Ok(directory) = File::open(parent) {
        let _ = directory.sync_all();
    }
}

#[cfg(test)]
mod tests;
