//! Filesystem transaction shared by user configuration and application state.
use super::ConfigError;
use atomic_write_file::AtomicWriteFile;
use std::{
    fs::{self, File},
    io::{self, Write},
    path::{Path, PathBuf},
};

/// Failure to persist user configuration or application state.
#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum ConfigWriteError {
    #[error(transparent)]
    Config(#[from] ConfigError),
    #[error("no user configuration path is available")]
    NoPath,
    #[error("application state needs a plain filename separate from the configuration file")]
    StateName,
    #[error("another Recite process is updating configuration or state file {path}")]
    Locked { path: PathBuf },
    #[error("configuration or state file must be a writable regular file: {path}")]
    NotWritable { path: PathBuf },
    #[error("configuration or state file changed during the update: {path}")]
    Conflict { path: PathBuf },
    #[error("could not {operation} configuration or state file {path}: {source}")]
    Io {
        path: PathBuf,
        operation: &'static str,
        #[source]
        source: io::Error,
    },
    #[error(
        "could not commit configuration or state file {path}; reload it to determine whether replacement completed: {source}"
    )]
    Commit {
        path: PathBuf,
        #[source]
        source: io::Error,
    },
    #[error("could not edit user config: {0}")]
    Toml(#[from] toml_edit::TomlError),
}

/// The OS lock lives as long as this transaction, including final replacement.
/// Its persistent sidecar avoids unlink/recreate races between cooperating writers.
pub(super) struct UserFileTransaction<'a> {
    path: &'a Path,
    _lock: File,
}

impl<'a> UserFileTransaction<'a> {
    pub(super) fn begin(path: &'a Path) -> Result<Self, ConfigWriteError> {
        let parent = path.parent().ok_or(ConfigWriteError::NoPath)?;
        fs::create_dir_all(parent).map_err(|error| failure(path, "create directory for", error))?;
        let mut lock_path = path.as_os_str().to_owned();
        lock_path.push(".lock");
        let lock = File::options()
            .create(true)
            .truncate(false)
            .read(true)
            .write(true)
            .open(PathBuf::from(lock_path))
            .map_err(|error| failure(path, "open lock for", error))?;
        match lock.try_lock() {
            Ok(()) => {}
            Err(fs::TryLockError::WouldBlock) => {
                return Err(ConfigWriteError::Locked {
                    path: path.to_owned(),
                });
            }
            Err(fs::TryLockError::Error(error)) => return Err(failure(path, "lock", error)),
        }
        writable_permissions(path)?;
        Ok(Self { path, _lock: lock })
    }

    pub(super) fn replace(
        self,
        expected: Option<&str>,
        replacement: &str,
    ) -> Result<(), ConfigWriteError> {
        let permissions = writable_permissions(self.path)?;
        if expected == Some(replacement) {
            return self.check_current(expected);
        }
        let mut output = AtomicWriteFile::open(self.path)
            .map_err(|error| failure(self.path, "prepare replacement for", error))?;
        if let Some(permissions) = permissions {
            output
                .as_file()
                .set_permissions(permissions)
                .map_err(|error| failure(self.path, "preserve permissions for", error))?;
        }
        output
            .write_all(replacement.as_bytes())
            .map_err(|error| failure(self.path, "write replacement for", error))?;
        self.check_current(expected)?;
        // commit syncs the file and performs the platform-specific replacement.
        // A commit error may be reported after replacement; callers must reload.
        output.commit().map_err(|source| ConfigWriteError::Commit {
            path: self.path.to_owned(),
            source,
        })
    }

    fn check_current(&self, expected: Option<&str>) -> Result<(), ConfigWriteError> {
        writable_permissions(self.path)?;
        let current = match fs::read_to_string(self.path) {
            Ok(source) => Some(source),
            Err(error) if error.kind() == io::ErrorKind::NotFound => None,
            Err(error) => return Err(failure(self.path, "recheck", error)),
        };
        if current.as_deref() != expected {
            return Err(ConfigWriteError::Conflict {
                path: self.path.to_owned(),
            });
        }
        Ok(())
    }
}

fn writable_permissions(path: &Path) -> Result<Option<fs::Permissions>, ConfigWriteError> {
    match fs::symlink_metadata(path) {
        Ok(metadata) if metadata.is_file() && !metadata.permissions().readonly() => {
            Ok(Some(metadata.permissions()))
        }
        Ok(_) => Err(ConfigWriteError::NotWritable {
            path: path.to_owned(),
        }),
        Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(None),
        Err(error) => Err(failure(path, "inspect", error)),
    }
}

fn failure(path: &Path, operation: &'static str, source: io::Error) -> ConfigWriteError {
    ConfigWriteError::Io {
        path: path.to_owned(),
        operation,
        source,
    }
}
