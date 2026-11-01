use super::*;
use recite_compiler::authoring::{CancellationToken, WorkControl};
impl ProjectFiles {
    /// Grouped history follows local field and document history in Undo/Redo ordering.
    pub fn has_rename_history(&self, current: &Workbench, redo: bool) -> bool {
        let journal = if redo {
            self.rename_redo.last()
        } else {
            self.rename_undo.last()
        };
        !current.has_draft()
            && journal.is_some_and(|journal| {
                journal.expected.get(&self.current).is_some_and(|source| {
                    if redo {
                        source == current.document().source()
                    } else {
                        !current.document().can_undo()
                    }
                })
            })
    }

    pub fn rename_history(
        &mut self,
        current: &mut Workbench,
        redo: bool,
    ) -> Result<bool, FileError> {
        self.rename_history_with_control(current, redo, &CancellationToken::new())
    }
    pub fn rename_history_with_control(
        &mut self,
        current: &mut Workbench,
        redo: bool,
        control: &dyn WorkControl,
    ) -> Result<bool, FileError> {
        self.require_navigation_ready()?;
        control.checkpoint()?;
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
        self.require_idle_save()?;
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
        control.checkpoint()?;
        let checkpoint = self
            .manifest
            .checkpoint(next_manifest, snapshots, affected)?;
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
        }
        journal.manifest = previous_manifest;
        journal.expected_manifest = self.manifest.text().to_owned();
        if redo {
            self.rename_undo.push(journal);
        } else {
            self.rename_redo.push(journal);
        }
        self.finish_project_edit(current)?;
        checkpoint.result()?;
        Ok(true)
    }
}
