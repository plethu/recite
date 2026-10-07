//! Restore durable project-edit journals before exposing any document session.
use super::{FileError, ProjectFiles};
use crate::recovery::RecoveryStore;
use recite_compiler::authoring::WorkControl;
use recite_writer_model::{Document, Workbench};

impl ProjectFiles {
    pub(crate) fn workbench_with_control(
        &mut self,
        control: &dyn WorkControl,
    ) -> Result<Workbench, FileError> {
        control.checkpoint()?;
        // Complete an interrupted multi-file checkpoint before restoring sessions.
        let pending = self.manifest.pending().clone();
        for (name, recovery) in pending {
            control.checkpoint()?;
            let path = self.path_for_document(&name).ok_or(FileError::Selection)?;
            if path == self.current {
                self.recovery.persist(Some(recovery))?;
            } else {
                RecoveryStore::open(&path)?.persist(Some(recovery))?;
            }
        }
        let recovered = self.recovery.snapshot();
        let source = recovered.map_or(self.saved.as_ref(), |r| r.draft.source());
        let key = recite_core::DocumentKey::new(self.document_name()?)
            .map_err(recite_writer_model::EditError::from)
            .map_err(recite_writer_model::WorkbenchError::from)?;
        let document =
            Document::in_project_with_control(key, source, self.context.clone(), control)
                .map_err(recite_writer_model::WorkbenchError::from)?;
        control.checkpoint()?;
        let mut workbench = Workbench::from_document(document)?;
        control.checkpoint()?;
        if let Some(recovery) = recovered {
            recovery.draft.restore(&mut workbench)?;
            self.saved = recovery.baseline.clone();
        }
        control.checkpoint()?;
        let original = self.current.clone();
        let affected = self.manifest.affected().to_vec();
        for name in affected {
            control.checkpoint()?;
            let path = self.path_for_document(&name).ok_or(FileError::Selection)?;
            self.switch_with_control(&mut workbench, &path, |_| Ok(()), control)?;
        }
        self.switch_with_control(&mut workbench, &original, |_| Ok(()), control)?;
        if !self.manifest.pending().is_empty() {
            self.manifest.checkpointed()?;
        }
        control.checkpoint()?;
        Ok(workbench)
    }
}

#[cfg(test)]
mod tests;
