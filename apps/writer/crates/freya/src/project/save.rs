use super::{FileError, ProjectFiles, read_regular};
use atomic_write_file::AtomicWriteFile;
use recite_writer_model::Workbench;
use std::{
    fs::{self, OpenOptions},
    io::{self, Write},
    path::{Path, PathBuf},
};

impl ProjectFiles {
    pub(crate) fn save_all(&mut self, current: &mut Workbench) -> Result<(), FileError> {
        let outcome = (|| {
            self.save_current(current)?;
            self.save_retained()?;
            if let Some(session) = &mut self.declarations
                && session.dirty()
            {
                session.save_and_generate()?;
                self.refresh(current)?;
            }
            Ok(())
        })();
        self.refresh_after_save(current, outcome)
    }

    /// Apply the active draft, save it durably and finish grouped project edits.
    pub(crate) fn save_current(&mut self, current: &mut Workbench) -> Result<(), FileError> {
        let outcome = (|| {
            current.apply()?;
            let grouped = self.project_edit_pending(current);
            self.save(current.document().source())?;
            self.checkpoint(current)?;
            if grouped {
                self.save_project_edit(current)?;
            }
            Ok(())
        })();
        self.refresh_after_save(current, outcome)
    }

    /// Saving another tab does not change the active scene or its editing state.
    pub(crate) fn save_document(
        &mut self,
        current: &mut Workbench,
        path: &Path,
    ) -> Result<(), FileError> {
        if path == self.current {
            return self.save_current(current);
        }
        let outcome = (|| {
            let grouped = self.project_edit_pending(current) && self.project_edit_includes(path);
            let session = self.retained.get_mut(path).ok_or(FileError::Selection)?;
            let outcome = session.save(path);
            let source = session.baseline.clone();
            let record = self.record_saved(path, &source);
            outcome.and(record)?;
            if grouped {
                self.save_project_edit(current)?;
            }
            Ok(())
        })();
        self.refresh_after_save(current, outcome)
    }

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

    /// Finish the already-applied project transaction; unrelated field drafts
    /// remain drafts, including the one in the active scene.
    fn save_project_edit(&mut self, current: &Workbench) -> Result<(), FileError> {
        if self.project_edit_includes(&self.current) {
            self.save(current.document().source())?;
            self.checkpoint(current)?;
        }
        let paths: Vec<_> = self
            .retained
            .keys()
            .filter(|path| self.project_edit_includes(path))
            .cloned()
            .collect();
        for path in paths {
            let session = self.retained.get_mut(&path).ok_or(FileError::Selection)?;
            let outcome = session.save_applied(&path);
            let source = session.baseline.clone();
            let record = self.record_saved(&path, &source);
            outcome.and(record)?;
        }
        self.manifest.save()
    }

    /// Cooperative lock + checked replacement. Each replacement retains a backup
    /// of the previous bytes. Non-cooperating writers can still race the final check.
    pub fn save(&mut self, source: &str) -> Result<(), FileError> {
        replace_checked(&self.current, &self.saved, source)?;
        self.saved = source.into();
        self.update_saved_context()
    }
}

impl ProjectFiles {
    pub(super) fn update_saved_context(&mut self) -> Result<(), FileError> {
        self.record_saved(&self.current.clone(), &self.saved.clone())
    }

    pub(super) fn record_saved(&mut self, path: &Path, source: &str) -> Result<(), FileError> {
        let key = recite_core::DocumentKey::new(self.names.get(path).ok_or(FileError::Selection)?)
            .map_err(recite_writer_model::EditError::from)
            .map_err(recite_writer_model::WorkbenchError::from)?;
        let document = recite_compiler::authoring::SavedDocument::new(key, source);
        if let Some(old) = self
            .context
            .documents
            .iter_mut()
            .find(|old| old.key() == document.key())
        {
            if old.text() == document.text() {
                return Ok(());
            }
            *old = document.clone();
        }
        std::sync::Arc::make_mut(&mut self.search).replace_document(document);
        Ok(())
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
