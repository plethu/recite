//! Retained documents preserve drafts, undo history and recovery ownership on navigation.
pub(super) mod checkpoint;
pub(super) mod preparing;
use super::{FileError, ProjectFiles};
use crate::recovery::{Recovery, RecoveryStore};
use recite_compiler::authoring::{CancellationToken, WorkControl};
use recite_writer_model::Workbench;
use std::{
    path::{Path, PathBuf},
    sync::Arc,
};

pub(super) struct Retained {
    pub(super) model: Workbench,
    pub(super) baseline: Arc<str>,
    pub(super) recovery: RecoveryStore,
}
impl ProjectFiles {
    /// Every session still owns its history and recovery state until it closes.
    pub(crate) fn session_paths(&self) -> impl Iterator<Item = &PathBuf> {
        std::iter::once(&self.current).chain(self.retained.keys())
    }

    pub(super) fn retained_context(
        &self,
        mut context: recite_writer_model::ProjectContext,
    ) -> recite_writer_model::ProjectContext {
        for session in self.retained.values() {
            overlay(&mut context, &session.model);
        }
        context
    }

    /// Catalogue extraction refreshes clean scenes from disk and overlays applied
    /// unsaved edits. Unapplied drafts must be resolved before extraction.
    pub(crate) fn catalogue_context(
        &self,
        mut context: recite_writer_model::ProjectContext,
    ) -> Result<recite_writer_model::ProjectContext, FileError> {
        for session in self.retained.values() {
            if session.model.has_draft() {
                return Err(FileError::UnsavedDocument);
            }
            if session.model.document().source() != session.baseline.as_ref() {
                overlay(&mut context, &session.model);
            }
        }
        Ok(context)
    }

    /// The current document and every retained session, in project order.
    pub fn open_documents(&self, current: &Workbench) -> Vec<(PathBuf, bool)> {
        let grouped = self.project_edit_pending(current);
        self.paths
            .iter()
            .filter_map(|path| {
                if path == &self.current {
                    Some((
                        path.clone(),
                        current.has_draft()
                            || self.dirty(current.document().source())
                            || (grouped && self.project_edit_includes(path)),
                    ))
                } else {
                    self.retained.get(path).map(|s| {
                        (
                            path.clone(),
                            s.dirty() || (grouped && self.project_edit_includes(path)),
                        )
                    })
                }
            })
            .collect()
    }

    /// Closing is deliberately limited to a saved session. The caller must save
    /// or explicitly resolve drafts first; navigation alone never discards them.
    pub fn close_document(
        &mut self,
        current: &mut Workbench,
        path: &Path,
    ) -> Result<(), FileError> {
        self.require_saved_document(current, path)?;
        let documents = self.open_documents(current);
        if path == self.current {
            let next = documents
                .iter()
                .find(|(p, _)| p != path)
                .ok_or(FileError::Selection)?;
            // A cleanup error must leave the old document active.
            self.recovery.persist(None)?;
            self.switch(current, &next.0, None, |_| Ok(()))?;
        }
        let session = self.retained.get_mut(path).ok_or(FileError::Selection)?;
        session.persist_recovery(None)?;
        self.forget_rename_history(path);
        self.retained.remove(path);
        Ok(())
    }

    pub(crate) fn require_saved_document(
        &self,
        current: &Workbench,
        path: &Path,
    ) -> Result<(), FileError> {
        self.require_idle_save()?;
        if !self.open_documents(current).iter().any(|(p, _)| p == path) {
            return Err(FileError::Selection);
        }
        let edits = if path == self.current {
            current.has_draft() || self.saved.as_ref() != current.document().source()
        } else {
            self.retained
                .get(path)
                .ok_or(FileError::Selection)?
                .has_edits()
        };
        if edits || (self.project_edit_pending(current) && self.project_edit_includes(path)) {
            return Err(FileError::UnsavedDocument);
        }
        Ok(())
    }

    pub fn retained_dirty(&self) -> bool {
        self.manifest.dirty() || self.retained.values().any(Retained::dirty)
    }

    pub(crate) fn can_leave(&self, current: &Workbench) -> bool {
        !self.saving
            && !current.has_draft()
            && !self.dirty(current.document().source())
            && !self.retained_dirty()
            && !self.builds.busy()
            && !self
                .declarations
                .as_ref()
                .is_some_and(|s| s.dirty() || s.busy())
    }

    pub fn switch(
        &mut self,
        current: &mut Workbench,
        path: &Path,
        initial: Option<recite_writer_model::View>,
        select: impl FnOnce(&mut Workbench) -> Result<(), recite_writer_model::WorkbenchError>,
    ) -> Result<(), FileError> {
        self.switch_with_control(current, path, initial, select, &CancellationToken::new())
    }

    pub(super) fn switch_with_control(
        &mut self,
        current: &mut Workbench,
        path: &Path,
        initial: Option<recite_writer_model::View>,
        select: impl FnOnce(&mut Workbench) -> Result<(), recite_writer_model::WorkbenchError>,
        control: &dyn WorkControl,
    ) -> Result<(), FileError> {
        self.require_idle_save()?;
        control.checkpoint()?;
        if path == self.current {
            select(current)?;
            return Ok(());
        }
        if !self.paths.iter().any(|candidate| candidate == path) {
            return Err(FileError::Selection);
        }
        let mut prepared = self
            .switch_request(current, path, initial)?
            .prepare(control, select)?;
        let checkpoint = self.begin_navigation_checkpoint(current)?.wait()?;
        control.checkpoint()?;
        self.accept_switch(current, &mut prepared, &checkpoint)?;
        Ok(())
    }
}

impl Retained {
    pub(super) fn persist_recovery(&mut self, recovery: Option<Recovery>) -> Result<(), FileError> {
        let persisted = self.recovery.persist(recovery);
        // Failed writes retain their error and lock, but need no idle thread.
        let parked = self.recovery.park();
        persisted.and(parked)
    }

    fn has_edits(&self) -> bool {
        self.model.has_draft() || self.model.document().source() != self.baseline.as_ref()
    }

    fn dirty(&self) -> bool {
        self.has_edits() || self.recovery.pending()
    }
}

fn overlay(context: &mut recite_writer_model::ProjectContext, model: &Workbench) {
    let saved = recite_compiler::authoring::SavedDocument::from_shared(
        model.document().key().clone(),
        model.document().source_snapshot(),
    );
    if let Some(slot) = context
        .documents
        .iter_mut()
        .find(|d| d.key() == saved.key())
    {
        *slot = saved;
    }
}

#[cfg(test)]
mod tests;
