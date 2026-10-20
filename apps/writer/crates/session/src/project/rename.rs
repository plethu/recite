//! Rename commits swap prebuilt sessions together; the journal retains their histories.
use super::{FileError, ProjectFiles, manifest::Checkpoint, read_regular};
mod applying;
mod history;
mod reviewing;
use recite_writer_model::{ProjectRename, Workbench};
use std::{collections::BTreeMap, path::PathBuf};

pub struct RenameReview {
    pub plan: ProjectRename,
    pub manifest_before: String,
    pub manifest_after: String,
    baseline: BTreeMap<PathBuf, String>,
    block: String,
    name: String,
}
pub(super) struct RenameUndo {
    models: BTreeMap<PathBuf, Workbench>,
    expected: BTreeMap<PathBuf, String>,
    manifest: String,
    expected_manifest: String,
}
impl ProjectFiles {
    pub(super) fn project_edit_includes(&self, path: &std::path::Path) -> bool {
        self.names
            .get(path)
            .is_some_and(|name| self.manifest.affected().contains(name))
            || self
                .rename_undo
                .iter()
                .chain(&self.rename_redo)
                .any(|journal| journal.expected.contains_key(path))
    }

    pub(super) fn forget_rename_history(&mut self, path: &std::path::Path) {
        // Project undo requires every affected session. Closing one ends that
        // history as a unit, rather than leaving a stale, partially usable chain.
        if self
            .rename_undo
            .iter()
            .chain(&self.rename_redo)
            .any(|journal| journal.expected.contains_key(path))
        {
            self.rename_undo.clear();
            self.rename_redo.clear();
        }
    }

    pub fn project_edit_pending(&self, current: &Workbench) -> bool {
        self.manifest.dirty()
            || self
                .rename_undo
                .iter()
                .chain(&self.rename_redo)
                .any(|journal| {
                    journal.expected.keys().any(|path| {
                        if path == &self.current {
                            current.has_draft()
                                || self.saved.as_ref() != current.document().source()
                        } else {
                            self.retained.get(path).is_some_and(|s| {
                                s.model.has_draft()
                                    || s.model.document().source() != s.baseline.as_ref()
                            })
                        }
                    })
                })
    }
}

impl ProjectFiles {
    fn checkpoint_project_edit(
        &mut self,
        text: String,
        models: &BTreeMap<PathBuf, Workbench>,
    ) -> Result<Checkpoint, FileError> {
        let snapshots = models
            .iter()
            .map(|(path, model)| {
                let baseline = if path == &self.current {
                    self.saved.clone()
                } else {
                    self.retained[path].baseline.clone()
                };
                Ok((
                    self.names.get(path).ok_or(FileError::Selection)?.clone(),
                    crate::recovery::Recovery::new(baseline, model.recovery()),
                ))
            })
            .collect::<Result<BTreeMap<_, _>, FileError>>()?;
        let mut affected = self.manifest.affected().to_vec();
        affected.extend(snapshots.keys().cloned());
        affected.sort();
        affected.dedup();
        self.manifest.checkpoint(text, snapshots, affected)
    }

    pub fn project_edit_recovery_pending(&self) -> bool {
        self.manifest.recovery_pending()
    }
    pub fn require_project_edit_ready(&self) -> Result<(), FileError> {
        if self.project_edit_recovery_pending() {
            Err(FileError::ProjectEditRecoveryPending)
        } else {
            Ok(())
        }
    }
    /// Complete the already-published grouped edit before admitting newer mutations.
    pub fn finish_project_edit(&mut self, current: &Workbench) -> Result<(), FileError> {
        self.finish_project_edit_with_sync(current, super::save::sync_directory)
    }
    fn finish_project_edit_with_sync(
        &mut self,
        current: &Workbench,
        sync: impl FnOnce(&std::path::Path) -> std::io::Result<()>,
    ) -> Result<(), FileError> {
        for (name, recovery) in self.manifest.pending() {
            let path = self.path_for_document(name).ok_or(FileError::Selection)?;
            let model = if path == self.current {
                current
            } else {
                &self
                    .retained
                    .get(&path)
                    .ok_or(FileError::ProjectEditRecoveryRestart)?
                    .model
            };
            if model.recovery() != recovery.draft {
                return Err(FileError::ProjectEditRecoveryRestart);
            }
        }
        self.recovery.persist(self.recovery_value(current))?;
        for session in self.retained.values_mut() {
            session.persist_recovery(Some(crate::recovery::Recovery::new(
                session.baseline.clone(),
                session.model.recovery(),
            )))?;
        }
        self.manifest.checkpointed_with_sync(sync)
    }
}

#[cfg(test)]
mod tests;
