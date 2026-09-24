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
        self.paths
            .iter()
            .filter(|p| !self.closed_tabs.contains(*p))
            .filter_map(|path| {
                if path == &self.current {
                    Some((
                        path.clone(),
                        current.has_draft() || self.dirty(current.document().source()),
                    ))
                } else {
                    self.retained.get(path).map(|s| {
                        (
                            path.clone(),
                            s.model.has_draft()
                                || s.model.document().source() != s.baseline.as_ref(),
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
        let (_, dirty) = documents
            .iter()
            .find(|(p, _)| p == path)
            .ok_or(FileError::Selection)?;
        if *dirty {
            return Err(FileError::UnsavedDocument);
        }
        if path == self.current {
            let next = documents
                .iter()
                .find(|(p, _)| p != path)
                .ok_or(FileError::Selection)?;
            self.switch(current, &next.0, |_| Ok(()))?;
        }
        self.closed_tabs.insert(path.to_owned());
        Ok(())
    }

    pub fn retained_dirty(&self) -> bool {
        self.manifest.dirty()
            || self
                .retained
                .values()
                .any(|s| s.model.has_draft() || s.model.document().source() != s.baseline.as_ref())
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
        self.closed_tabs.remove(path);
        self.retained.insert(previous, next);
        Ok(())
    }

    /// Every retained session was checkpointed before it stopped being active.
    /// Saving one failure leaves the remaining sessions and current selection intact.
    pub fn save_retained(&mut self) -> Result<(), FileError> {
        let paths: Vec<PathBuf> = self.retained.keys().cloned().collect();
        for path in paths {
            let mut session = self.retained.remove(&path).ok_or(FileError::Selection)?;
            let result = (|| {
                session.model.apply()?;
                let previous_path = std::mem::replace(&mut self.current, path.clone());
                std::mem::swap(&mut self.saved, &mut session.baseline);
                std::mem::swap(&mut self.recovery, &mut session.recovery);
                let result = self
                    .save(session.model.document().source())
                    .and_then(|()| self.checkpoint(&session.model));
                std::mem::swap(&mut self.saved, &mut session.baseline);
                std::mem::swap(&mut self.recovery, &mut session.recovery);
                self.current = previous_path;
                result
            })();
            self.retained.insert(path, session);
            result?;
        }
        if self.manifest.dirty() {
            self.manifest.save()?;
        }
        Ok(())
    }
}

fn overlay(context: &mut recite_writer_model::ProjectContext, model: &Workbench) {
    let saved = recite_compiler::SavedDocument::new(
        model.document().key().clone(),
        model.document().source(),
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
