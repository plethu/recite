//! Save preparation and receipts keep editor state separate from durable writes.
use super::super::{FileError, ProjectFiles};
use crate::recovery::{Recovery, worker::Durability};
use recite_writer_model::Workbench;
use std::{path::PathBuf, sync::Arc};

pub(crate) enum Scope {
    Current,
    Document(PathBuf),
    All,
    Declarations,
}
impl Scope {
    pub(crate) fn applies_current(&self, current: &std::path::Path) -> bool {
        match self {
            Self::Current | Self::All => true,
            Self::Document(path) => path == current,
            Self::Declarations => false,
        }
    }
}
#[derive(Clone, Copy)]
pub(crate) enum Drafts {
    Apply,
    Retain,
}
pub(crate) enum Step {
    Current(Arc<str>),
    Retained { path: PathBuf, drafts: Drafts },
    Manifest,
    Declarations { force: bool },
}
pub(crate) struct Write {
    pub path: PathBuf,
    pub baseline: Arc<str>,
    pub source: Arc<str>,
    document: recite_core::DocumentKey,
    search: Arc<recite_writer_model::SearchIndex>,
}
impl ProjectFiles {
    pub(crate) fn set_saving(&mut self, saving: bool) {
        self.saving = saving;
        if let Some(session) = &mut self.declarations {
            session.set_saving(saving);
        }
    }
    pub(crate) fn require_idle_save(&self) -> Result<(), FileError> {
        if self.saving {
            Err(FileError::SavePending)
        } else {
            Ok(())
        }
    }
    pub(crate) fn save_plan_accepted(
        &self,
        current: &Workbench,
        scope: Scope,
    ) -> Result<Vec<Step>, FileError> {
        if matches!(scope, Scope::Declarations) {
            return Ok(vec![Step::Declarations { force: true }]);
        }
        let path = match &scope {
            Scope::Document(path) => path.clone(),
            Scope::Current | Scope::All => self.current.clone(),
            Scope::Declarations => unreachable!(),
        };
        let source = current.document().source_snapshot();
        let grouped = self.project_edit_pending(current)
            && (path == self.current || self.project_edit_includes(&path));
        let step = |path: &PathBuf, drafts| {
            if path == &self.current {
                Step::Current(source.clone())
            } else {
                Step::Retained {
                    path: path.clone(),
                    drafts,
                }
            }
        };
        let mut steps = vec![step(&path, Drafts::Apply)];
        if grouped {
            if path != self.current && self.project_edit_includes(&self.current) {
                steps.push(step(&self.current, Drafts::Retain));
            }
            for other in self
                .retained
                .keys()
                .filter(|p| *p != &path && self.project_edit_includes(p))
            {
                steps.push(step(other, Drafts::Retain));
            }
            steps.push(Step::Manifest);
        }
        if matches!(scope, Scope::All) {
            // A grouped save uses applied text first. Save All subsequently accepts
            // retained field drafts too, in the same order as synchronous saving.
            for path in self.retained.keys() {
                steps.push(step(path, Drafts::Apply));
            }
            steps.extend([Step::Manifest, Step::Declarations { force: false }]);
        }
        Ok(steps)
    }
    pub(crate) fn retained_edit(
        &self,
        step: &Step,
    ) -> Result<Option<(PathBuf, recite_writer_model::EditRequest)>, FileError> {
        let Step::Retained {
            path,
            drafts: Drafts::Apply,
        } = step
        else {
            return Ok(None);
        };
        let model = &self.retained.get(path).ok_or(FileError::Selection)?.model;
        Ok(model
            .edit_request(recite_writer_model::EditOperation::Apply)?
            .map(|r| (path.clone(), r)))
    }
    pub(crate) fn accept_retained_edit(
        &mut self,
        path: &std::path::Path,
        prepared: &mut recite_writer_model::PreparedEdit,
    ) -> Result<(), FileError> {
        self.retained
            .get_mut(path)
            .ok_or(FileError::Selection)?
            .model
            .accept_edit(prepared)
            .map_err(FileError::from)
    }
    pub(crate) fn prepare_write(&mut self, step: Step) -> Result<Write, FileError> {
        let path = match &step {
            Step::Current(_) => &self.current,
            Step::Retained { path, .. } => path,
            _ => return Err(FileError::Selection),
        };
        let document =
            recite_core::DocumentKey::new(self.names.get(path).ok_or(FileError::Selection)?)
                .map_err(recite_writer_model::EditError::from)
                .map_err(recite_writer_model::WorkbenchError::from)?;
        let search = self.search.clone();
        match step {
            Step::Current(source) => Ok(Write {
                path: self.current.clone(),
                baseline: self.saved.clone(),
                source,
                document,
                search,
            }),
            Step::Retained { path, drafts } => {
                let session = self.retained.get_mut(&path).ok_or(FileError::Selection)?;
                if matches!(drafts, Drafts::Apply) {
                    session.model.apply()?;
                }
                Ok(Write {
                    path,
                    baseline: session.baseline.clone(),
                    source: session.model.document().source_snapshot(),
                    document,
                    search,
                })
            }
            Step::Manifest | Step::Declarations { .. } => Err(FileError::Selection),
        }
    }
    /// A successful receipt advances the saved baseline, even after newer edits.
    pub(crate) fn publish_write(
        &mut self,
        current: &Workbench,
        write: &mut Write,
    ) -> Result<Durability<Recovery>, FileError> {
        let snapshot = if write.path == self.current {
            self.saved = write.source.clone();
            let recovery = (current.has_draft()
                || self.saved.as_ref() != current.document().source())
            .then(|| Recovery::new(self.saved.clone(), current.recovery()));
            self.recovery.begin_persist(recovery)
        } else {
            let session = self
                .retained
                .get_mut(&write.path)
                .ok_or(FileError::Selection)?;
            session.baseline = write.source.clone();
            let recovery = (session.model.has_draft()
                || session.baseline.as_ref() != session.model.document().source())
            .then(|| Recovery::new(session.baseline.clone(), session.model.recovery()));
            session.recovery.begin_persist(recovery)
        };
        // Publish saved navigation even if recovery submission failed.
        let record = self.record_saved_document(&write.path, &write.source);
        // Keep the displaced project-sized index in the receipt for worker retirement.
        std::mem::swap(&mut self.search, &mut write.search);
        snapshot.and_then(|ticket| record.map(|_| ticket))
    }
    pub(crate) fn manifest_save(&self) -> Option<super::super::manifest::ManifestSave> {
        self.manifest.dirty().then(|| self.manifest.save_request())
    }
    pub(crate) fn accept_manifest_save(
        &mut self,
        outcome: super::super::manifest::ManifestOutcome,
    ) -> Result<(), FileError> {
        self.manifest.accept_save(outcome)
    }
    pub(crate) fn finish_saved_tab(&mut self, path: &std::path::Path) -> Result<(), FileError> {
        if let Some(session) = self.retained.get_mut(path) {
            session.recovery.park()?;
        }
        Ok(())
    }
    pub(crate) fn finish_save(
        &self,
        current: &mut Workbench,
        result: Result<(), FileError>,
    ) -> Result<(), FileError> {
        self.refresh_after_save(current, result)
    }
}

impl Write {
    pub(crate) fn execute(&mut self) -> Result<(), FileError> {
        super::replace_checked(&self.path, &self.baseline, &self.source)?;
        if self.baseline != self.source {
            Arc::make_mut(&mut self.search).replace_document(
                recite_compiler::authoring::SavedDocument::from_shared(
                    self.document.clone(),
                    self.source.clone(),
                ),
            );
        }
        Ok(())
    }
}

#[cfg(test)]
mod tests;
