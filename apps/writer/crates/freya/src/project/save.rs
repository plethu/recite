use super::{FileError, ProjectFiles, read_regular};
use atomic_write_file::AtomicWriteFile;
use recite_writer_model::Workbench;
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};
pub(crate) mod ordered;

impl ProjectFiles {
    fn refresh_after_save(
        &self,
        current: &mut Workbench,
        outcome: Result<(), FileError>,
    ) -> Result<(), FileError> {
        // Applying a draft or publishing earlier files can succeed before a later
        // save fails. Refresh navigation/diagnostics even then; retain the save error.
        let refresh = current
            .refresh_project(self.retained_context(self.context.clone()))
            .map_err(FileError::from);
        outcome.and(refresh)
    }
}

impl ProjectFiles {
    pub(super) fn update_saved_context(&mut self) -> Result<(), FileError> {
        self.record_saved(&self.current.clone(), &self.saved.clone())
    }

    pub(super) fn record_saved(
        &mut self,
        path: &Path,
        source: &std::sync::Arc<str>,
    ) -> Result<(), FileError> {
        if let Some(document) = self.record_saved_document(path, source)? {
            std::sync::Arc::make_mut(&mut self.search).replace_document(document);
        }
        Ok(())
    }
    pub(super) fn record_saved_document(
        &mut self,
        path: &Path,
        source: &std::sync::Arc<str>,
    ) -> Result<Option<recite_compiler::authoring::SavedDocument>, FileError> {
        let key = recite_core::DocumentKey::new(self.names.get(path).ok_or(FileError::Selection)?)
            .map_err(recite_writer_model::EditError::from)
            .map_err(recite_writer_model::WorkbenchError::from)?;
        if let Some(old) = self
            .context
            .documents
            .iter_mut()
            .find(|old| old.key() == &key)
        {
            if old.text() == source.as_ref() {
                return Ok(None);
            }
            *old =
                recite_compiler::authoring::SavedDocument::from_shared(key.clone(), source.clone());
        }
        let document = recite_compiler::authoring::SavedDocument::from_shared(key, source.clone());
        Ok(Some(document))
    }
}

/// Checked replacement shared by manuscript and standalone declaration sources.
pub(crate) fn replace_checked(path: &Path, baseline: &str, source: &str) -> Result<(), FileError> {
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

pub(super) fn sync_directory(parent: &Path) -> io::Result<()> {
    #[cfg(unix)]
    fs::File::open(parent)?.sync_all()?;
    #[cfg(not(unix))]
    let _ = parent;
    Ok(())
}

#[cfg(test)]
mod tests;
