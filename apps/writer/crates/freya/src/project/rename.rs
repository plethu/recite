//! Rename commits swap prebuilt sessions together; the journal retains their histories.
use super::{FileError, ProjectFiles};
use recite_writer_model::{Document, ProjectRename, Workbench};
use std::{collections::BTreeMap, path::PathBuf};

pub(crate) struct RenameReview {
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
    pub fn project_edit_pending(&self, current: &Workbench) -> bool {
        self.manifest.dirty()
            || self
                .rename_undo
                .iter()
                .chain(&self.rename_redo)
                .any(|journal| {
                    journal.expected.keys().any(|path| {
                        if path == &self.current {
                            current.has_draft() || self.dirty(current.document().source())
                        } else {
                            self.retained.get(path).is_some_and(|s| {
                                s.model.has_draft()
                                    || s.model.document().source() != s.baseline.as_ref()
                            })
                        }
                    })
                })
    }

    pub fn review_rename(
        &mut self,
        model: &mut Workbench,
        block: &str,
        name: &str,
    ) -> Result<(), FileError> {
        if model.has_draft() || self.retained.values().any(|s| s.model.has_draft()) {
            return Err(FileError::UnsavedDocument);
        }
        self.refresh(model)?;
        let plan = model
            .document()
            .plan_project_rename(block, name)
            .map_err(recite_writer_model::WorkbenchError::from)?;
        let mut baseline = self
            .paths
            .iter()
            .map(|p| super::read_regular(p).map(|source| (p.clone(), source)))
            .collect::<Result<BTreeMap<_, _>, _>>()?;
        let manifest_path = self.root.join("recite.project.toml");
        baseline.insert(manifest_path.clone(), super::read_regular(&manifest_path)?);
        let manifest_before = self.manifest.text().to_owned();
        let manifest_after =
            recite_writer_model::rename_manifest_source(&manifest_before, block, name)
                .map_err(recite_writer_model::WorkbenchError::from)?;
        self.rename_review = Some(RenameReview {
            manifest_before,
            manifest_after,
            plan,
            baseline,
            block: block.into(),
            name: name.into(),
        });
        Ok(())
    }
    pub fn apply_rename(&mut self, current: &mut Workbench) -> Result<(), FileError> {
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
            if super::read_regular(path)? != *text {
                return Err(FileError::Conflict);
            }
        }
        let block = review.block.clone();
        let name = review.name.clone();
        let expected = review.plan.changes.clone();
        self.refresh(current)?;
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
            if let Err(e) = self.switch(current, &path, |_| Ok(())) {
                self.switch(current, &original, |_| Ok(()))?;
                return Err(e);
            }
        }
        self.switch(current, &original, |_| Ok(()))?;
        let mut context = self.retained_context(self.context.clone());
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
            let document =
                Document::in_project(change.document.clone(), &change.after, context.clone())
                    .map_err(recite_writer_model::WorkbenchError::from)?;
            let mut model = Workbench::from_document(document)?;
            model.select(recite_writer_model::View::Source)?;
            expected.insert(path.clone(), change.after.clone());
            models.insert(path, model);
        }
        self.checkpoint_project_edit(manifest_after.clone(), &models)?;
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
        self.checkpoint(current)?;
        for session in self.retained.values_mut() {
            session
                .recovery
                .persist(Some(crate::recovery::Recovery::new(
                    session.baseline.clone(),
                    session.model.recovery(),
                )))?;
        }
        self.manifest.checkpointed()?;
        Ok(())
    }
    pub fn rename_history(
        &mut self,
        current: &mut Workbench,
        redo: bool,
    ) -> Result<bool, FileError> {
        let Some(journal) = (if redo {
            self.rename_redo.last()
        } else {
            self.rename_undo.last()
        }) else {
            return Ok(false);
        };
        if !journal.expected.contains_key(&self.current) {
            return Ok(false);
        }
        if !redo && current.document().can_undo() {
            return Ok(false);
        }
        if redo
            && journal
                .expected
                .get(&self.current)
                .is_some_and(|s| s != current.document().source())
        {
            return Ok(false);
        }
        if self.manifest.text() != journal.expected_manifest {
            return Err(FileError::StaleRename);
        }
        let previous_manifest = self.manifest.text().to_owned();
        let next_manifest = journal.manifest.clone();
        for (path, source) in &journal.expected {
            let model = if path == &self.current {
                &*current
            } else {
                &self.retained.get(path).ok_or(FileError::StaleRename)?.model
            };
            if model.has_draft() || model.document().source() != source {
                return Err(FileError::StaleRename);
            }
        }
        let snapshots = journal
            .models
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
        self.manifest.set(next_manifest, snapshots, affected)?;
        let mut journal = (if redo {
            self.rename_redo.pop()
        } else {
            self.rename_undo.pop()
        })
        .ok_or(FileError::StaleRename)?;
        for (path, replacement) in &mut journal.models {
            let model = if path == &self.current {
                &mut *current
            } else {
                &mut self
                    .retained
                    .get_mut(path)
                    .ok_or(FileError::StaleRename)?
                    .model
            };
            std::mem::swap(model, replacement);
            journal
                .expected
                .insert(path.clone(), model.document().source().into());
            self.closed_tabs.remove(path);
        }
        journal.manifest = previous_manifest;
        journal.expected_manifest = self.manifest.text().to_owned();
        if redo {
            self.rename_undo.push(journal);
        } else {
            self.rename_redo.push(journal);
        }
        self.checkpoint(current)?;
        for session in self.retained.values_mut() {
            session
                .recovery
                .persist(Some(crate::recovery::Recovery::new(
                    session.baseline.clone(),
                    session.model.recovery(),
                )))?;
        }
        self.manifest.checkpointed()?;
        Ok(true)
    }
}

impl ProjectFiles {
    fn checkpoint_project_edit(
        &mut self,
        text: String,
        models: &BTreeMap<PathBuf, Workbench>,
    ) -> Result<(), FileError> {
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
        self.manifest.set(text, snapshots, affected)
    }
}
