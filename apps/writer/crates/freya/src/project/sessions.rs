//! Retained documents preserve drafts, undo history and recovery ownership on navigation.
use super::{FileError, ProjectFiles, read_regular};
use crate::recovery::RecoveryStore;
use recite_writer_model::{Document, Workbench};
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
        let documents = self.open_documents(current);
        documents
            .iter()
            .find(|(p, _)| p == path)
            .ok_or(FileError::Selection)?;
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
        if path == self.current {
            let next = documents
                .iter()
                .find(|(p, _)| p != path)
                .ok_or(FileError::Selection)?;
            // A cleanup error must leave the old document active: the widgets
            // still contain its text until the caller receives a successful close.
            self.recovery.persist(None)?;
            self.switch(current, &next.0, |_| Ok(()))?;
        }
        // A clean closed tab has no draft to recover. Release its worker, lock,
        // compiler caches and local undo history only after cleanup succeeds.
        let session = self.retained.get_mut(path).ok_or(FileError::Selection)?;
        session.recovery.persist(None)?;
        self.forget_rename_history(path);
        self.retained.remove(path);
        Ok(())
    }

    pub fn retained_dirty(&self) -> bool {
        self.manifest.dirty() || self.retained.values().any(Retained::dirty)
    }

    pub(crate) fn can_leave(&self, current: &Workbench) -> bool {
        !current.has_draft()
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
        select: impl FnOnce(&mut Workbench) -> Result<(), recite_writer_model::WorkbenchError>,
    ) -> Result<(), FileError> {
        if path == self.current {
            select(current)?;
            return Ok(());
        }
        if !self.paths.iter().any(|candidate| candidate == path) {
            return Err(FileError::Selection);
        }
        // Navigation cannot leave a draft without a confirmed recovery snapshot.
        self.checkpoint(current)?;
        let mut context = self.retained_context(self.context.clone());
        overlay(&mut context, current);
        let mut next = if let Some(session) = self.retained.get_mut(path) {
            session.model.refresh_project(context)?;
            select(&mut session.model)?;
            self.retained.remove(path).ok_or(FileError::Selection)?
        } else {
            let mut baseline: Arc<str> = read_regular(path)?.into();
            let recovery = RecoveryStore::open(path)?;
            let key =
                recite_core::DocumentKey::new(self.names.get(path).ok_or(FileError::Selection)?)
                    .map_err(recite_writer_model::EditError::from)
                    .map_err(recite_writer_model::WorkbenchError::from)?;
            let source = recovery
                .snapshot()
                .map_or(baseline.as_ref(), |r| r.draft.source());
            let document = Document::in_project(key, source, context)
                .map_err(recite_writer_model::WorkbenchError::from)?;
            let mut model = Workbench::from_document(document)?;
            if let Some(snapshot) = recovery.snapshot() {
                snapshot.draft.restore(&mut model)?;
                baseline = snapshot.baseline.clone();
            }
            select(&mut model)?;
            Retained {
                model,
                baseline,
                recovery,
            }
        };
        std::mem::swap(current, &mut next.model);
        std::mem::swap(&mut self.saved, &mut next.baseline);
        std::mem::swap(&mut self.recovery, &mut next.recovery);
        let previous = std::mem::replace(&mut self.current, path.to_owned());
        self.retained.insert(previous, next);
        Ok(())
    }

    /// Every retained session was checkpointed before it stopped being active.
    /// Saving one failure leaves the remaining sessions and current selection intact.
    pub fn save_retained(&mut self) -> Result<(), FileError> {
        let paths: Vec<PathBuf> = self.retained.keys().cloned().collect();
        for path in paths {
            let session = self.retained.get_mut(&path).ok_or(FileError::Selection)?;
            let outcome = session.save(&path);
            let source = session.baseline.clone();
            let record = self.record_saved(&path, &source);
            outcome.and(record)?;
        }
        if self.manifest.dirty() {
            self.manifest.save()?;
        }
        Ok(())
    }
}

impl Retained {
    fn has_edits(&self) -> bool {
        self.model.has_draft() || self.model.document().source() != self.baseline.as_ref()
    }

    fn dirty(&self) -> bool {
        self.has_edits() || self.recovery.pending()
    }

    pub(super) fn save(&mut self, path: &Path) -> Result<(), FileError> {
        self.model.apply()?;
        self.save_applied(path)
    }

    pub(super) fn save_applied(&mut self, path: &Path) -> Result<(), FileError> {
        super::save::replace_checked(path, &self.baseline, self.model.document().source())?;
        self.baseline = self.model.document().source_snapshot();
        let recovery = self
            .model
            .has_draft()
            .then(|| crate::recovery::Recovery::new(self.baseline.clone(), self.model.recovery()));
        self.recovery.persist(recovery)
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
