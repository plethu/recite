//! Shared transactional text storage; callers own authority and schema validation.
use super::{ConfigWriteError, storage::UserFileTransaction};
use std::{fs, io, path::PathBuf};

#[derive(Clone, Debug)]
pub struct TextFileStore {
    path: PathBuf,
}

#[derive(Debug, thiserror::Error)]
#[non_exhaustive]
pub enum StateUpdateError<E: std::error::Error + 'static> {
    #[error(transparent)]
    Storage(#[from] ConfigWriteError),
    #[error("invalid replacement: {0}")]
    Edit(E),
}

impl TextFileStore {
    pub fn new(path: PathBuf) -> Self {
        Self { path }
    }

    pub fn load(&self) -> Result<Option<String>, ConfigWriteError> {
        match fs::symlink_metadata(&self.path) {
            Ok(metadata) if !metadata.is_file() => {
                return Err(ConfigWriteError::NotWritable {
                    path: self.path.clone(),
                });
            }
            Err(error) if error.kind() == io::ErrorKind::NotFound => return Ok(None),
            Err(source) => {
                return Err(ConfigWriteError::Io {
                    path: self.path.clone(),
                    operation: "inspect",
                    source,
                });
            }
            _ => {}
        }
        fs::read_to_string(&self.path)
            .map(Some)
            .map_err(|source| ConfigWriteError::Io {
                path: self.path.clone(),
                operation: "read",
                source,
            })
    }

    /// Reload under the shared OS lock and replace atomically. Invalid or future
    /// formats can be rejected by the caller without touching the original bytes.
    pub fn update<E: std::error::Error + 'static>(
        &self,
        edit: impl FnOnce(Option<&str>) -> Result<String, E>,
    ) -> Result<(), StateUpdateError<E>> {
        let transaction = UserFileTransaction::begin(&self.path)?;
        let previous = self.load()?;
        let replacement = edit(previous.as_deref()).map_err(StateUpdateError::Edit)?;
        transaction.replace(previous.as_deref(), &replacement)?;
        Ok(())
    }
}

/// User-owned application state uses the shared text-file transaction.
pub type UserStateFile = TextFileStore;
