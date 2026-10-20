use super::{FileError, ProjectFiles};
pub use crate::{replace_checked, sync_directory};
use recite_writer_model::Workbench;
use std::path::Path;
mod blocking;
pub mod ordered;
pub mod sequence;

impl ProjectFiles {
    fn refresh_after_save(
        &self,
        current: &mut Workbench,
        outcome: Result<(), FileError>,
    ) -> Result<(), FileError> {
        // Applying a draft or publishing earlier files can succeed before a later
        // save fails. Refresh navigation/diagnostics even then; retain the save error.
        let refresh = current
            .refresh_project(self.retained_context(self.context.as_ref().clone()))
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
        if let Some(index) = self
            .context
            .documents
            .iter()
            .position(|old| old.key() == &key)
        {
            if self.context.documents[index].text() == source.as_ref() {
                return Ok(None);
            }
            std::sync::Arc::make_mut(&mut self.context).documents[index] =
                recite_compiler::authoring::SavedDocument::from_shared(key.clone(), source.clone());
        }
        let document = recite_compiler::authoring::SavedDocument::from_shared(key, source.clone());
        Ok(Some(document))
    }
}

#[cfg(test)]
mod tests;
