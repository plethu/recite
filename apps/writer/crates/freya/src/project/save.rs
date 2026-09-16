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
        if old != self.saved.as_ref() {
            return Err(FileError::Conflict);
        }
        if source == self.saved.as_ref() {
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
        if read_regular(&self.current)? != self.saved.as_ref() {
            return Err(FileError::Conflict);
        }
        backup.keep().map_err(|error| error.error)?;
        replacement.commit().map_err(FileError::Commit)?;
        self.saved = source.into();
        #[cfg(unix)]
        fs::File::open(parent)?.sync_all()?;
        self.update_saved_context()?;
        Ok(())
    }
}

impl ProjectFiles {
    pub(super) fn update_saved_context(&mut self) -> Result<(), FileError> {
        let key = recite_core::DocumentKey::new(self.document_name()?)
            .map_err(recite_writer_model::EditError::from)
            .map_err(recite_writer_model::WorkbenchError::from)?;
        let document = recite_compiler::SavedDocument::new(key, self.saved.to_string());
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
