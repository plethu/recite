use super::*;
use recite_compiler::authoring::{CancellationToken, WorkControl};
use recite_writer_model::Document;
impl ProjectFiles {
    pub fn apply_rename(&mut self, current: &mut Workbench) -> Result<(), FileError> {
        self.apply_rename_with_control(current, &CancellationToken::new())
    }
    pub fn apply_rename_with_control(
        &mut self,
        current: &mut Workbench,
        control: &dyn WorkControl,
    ) -> Result<(), FileError> {
        self.require_idle_save()?;
        self.require_navigation_ready()?;
        control.checkpoint()?;
        if current.has_draft() || self.retained.values().any(|s| s.model.has_draft()) {
            return Err(FileError::UnsavedDocument);
        }
        let review = self.rename_review.as_ref().ok_or(FileError::Selection)?;
        if self.manifest.text() != review.manifest_before {
            return Err(FileError::StaleRename);
        }
        let manifest_before = review.manifest_before.clone();
        let manifest_after = review.manifest_after.clone();
        for (path, text) in &review.baseline {
            control.checkpoint()?;
            if super::read_regular(path)? != *text {
                return Err(FileError::Conflict);
            }
        }
        let block = review.block.clone();
        let name = review.name.clone();
        let expected = review.plan.changes.clone();
        self.refresh(current)?;
        control.checkpoint()?;
        let latest = current
            .document()
            .plan_project_rename(&block, &name)
            .map_err(recite_writer_model::WorkbenchError::from)?;
        if latest.changes != expected || self.manifest.text() != manifest_before {
            return Err(FileError::StaleRename);
        }
        // Acquire recovery ownership of every affected file before constructing replacements.
        let original = self.current.clone();
        for change in &latest.changes {
            let path = self
                .path_for_document(change.document.as_str())
                .ok_or(FileError::Selection)?;
            if let Err(e) = self.switch(current, &path, None, |_| Ok(())) {
                self.switch(current, &original, None, |_| Ok(()))?;
                return Err(e);
            }
            if current.has_draft() || current.document().source() != change.before {
                self.switch(current, &original, None, |_| Ok(()))?;
                return Err(FileError::RecoveredRenameInput(path));
            }
        }
        self.switch(current, &original, None, |_| Ok(()))?;
        let mut context = self.retained_context(self.context.as_ref().clone());
        for change in &latest.changes {
            if let Some(source) = context
                .documents
                .iter_mut()
                .find(|d| d.key() == &change.document)
            {
                *source = recite_compiler::authoring::SavedDocument::new(
                    change.document.clone(),
                    &change.after,
                );
            }
        }
        let mut models = BTreeMap::new();
        let mut expected = BTreeMap::new();
        for change in &latest.changes {
            let path = self
                .path_for_document(change.document.as_str())
                .ok_or(FileError::Selection)?;
            control.checkpoint()?;
            let previous = if path == self.current {
                &*current
            } else {
                &self.retained.get(&path).ok_or(FileError::Selection)?.model
            };
            let view = match previous.view() {
                recite_writer_model::View::Block(id)
                    if change.document == *current.document().key() && id == &block =>
                {
                    recite_writer_model::View::Block(name.clone())
                }
                view => view.clone(),
            };
            let document = Document::in_project_with_control(
                change.document.clone(),
                &change.after,
                context.clone(),
                control,
            )
            .map_err(recite_writer_model::WorkbenchError::from)?;
            let model = Workbench::from_document(document, Some(view))?;
            expected.insert(path.clone(), change.after.clone());
            models.insert(path, model);
        }
        control.checkpoint()?;
        let checkpoint = self.checkpoint_project_edit(manifest_after.clone(), &models)?;
        // No fallible operation between these swaps.
        for (path, replacement) in &mut models {
            if path == &self.current {
                std::mem::swap(current, replacement);
            } else if let Some(session) = self.retained.get_mut(path) {
                std::mem::swap(&mut session.model, replacement);
            }
        }
        self.rename_redo.clear();
        self.rename_undo.push(RenameUndo {
            models,
            expected,
            manifest: manifest_before,
            expected_manifest: manifest_after,
        });
        self.rename_review = None;
        self.finish_project_edit(current)?;
        checkpoint.result()
    }
}
