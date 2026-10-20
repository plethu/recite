//! An owned project and borrowed document views share one lifecycle and baseline.
use super::{FileError, ProjectFiles};
use crate::recovery::RecoveryStore;
use recite_compiler::authoring::CancellationToken;
use recite_core::DocumentKey;
use recite_writer_model::Workbench;
use std::path::{Path, PathBuf};

pub struct ProjectDocument {
    pub path: PathBuf,
    pub key: DocumentKey,
}

pub struct ProjectSession {
    files: ProjectFiles,
    model: Workbench,
    documents: Vec<ProjectDocument>,
}

/// A view borrows the actual retained owner; it never acquires another lease.
pub struct DocumentSession<'a> {
    path: &'a Path,
    model: &'a Workbench,
    baseline: &'a str,
    recovery: &'a RecoveryStore,
}

pub struct DocumentSessionMut<'a> {
    pub(super) files: &'a mut ProjectFiles,
    pub(super) model: &'a mut Workbench,
}

impl ProjectSession {
    pub fn open(project: &Path, requested: Option<&Path>) -> Result<Self, FileError> {
        let mut files = ProjectFiles::open_requested_with_control(
            project,
            requested,
            &CancellationToken::new(),
        )?;
        let model = files.workbench()?;
        let documents = documents(&files)?;
        Ok(Self {
            files,
            model,
            documents,
        })
    }
    pub fn root(&self) -> &Path {
        self.files.root()
    }
    pub fn documents(&self) -> &[ProjectDocument] {
        &self.documents
    }
    pub fn files(&self) -> &ProjectFiles {
        &self.files
    }
    pub fn parts_mut(&mut self) -> (&mut ProjectFiles, &mut Workbench) {
        (&mut self.files, &mut self.model)
    }
    pub fn current(&self) -> DocumentSession<'_> {
        DocumentSession {
            path: &self.files.current,
            model: &self.model,
            baseline: &self.files.saved,
            recovery: &self.files.recovery,
        }
    }
    pub fn current_mut(&mut self) -> DocumentSessionMut<'_> {
        DocumentSessionMut {
            files: &mut self.files,
            model: &mut self.model,
        }
    }
    pub fn sessions(&self) -> impl Iterator<Item = DocumentSession<'_>> {
        std::iter::once(self.current()).chain(self.files.retained.iter().map(|(path, session)| {
            DocumentSession {
                path,
                model: &session.model,
                baseline: &session.baseline,
                recovery: &session.recovery,
            }
        }))
    }
    pub fn dirty(&self) -> bool {
        self.current().dirty()
            || self.files.retained_dirty()
            || self
                .files
                .declarations
                .as_ref()
                .is_some_and(|session| session.dirty())
    }
    pub fn switch(&mut self, path: &Path) -> Result<(), FileError> {
        self.files.switch(&mut self.model, path, None, |_| Ok(()))
    }
    pub fn close_document(&mut self, path: &Path) -> Result<(), FileError> {
        self.files.close_document(&mut self.model, path)
    }
    /// Parking joins recovery workers without releasing any document leases.
    pub fn checkpoint(&mut self) -> Result<(), FileError> {
        self.files
            .begin_navigation_checkpoint(&self.model)?
            .wait()?;
        for session in self.files.retained.values_mut() {
            let recovery = (session.model.has_draft()
                || session.model.document().source() != session.baseline.as_ref())
            .then(|| {
                crate::recovery::Recovery::new(session.baseline.clone(), session.model.recovery())
            });
            session.persist_recovery(recovery)?;
        }
        Ok(())
    }
    pub fn save_all(&mut self) -> Result<(), FileError> {
        self.files.save_all(&mut self.model)?;
        self.checkpoint()
    }
    pub fn reload_current(&mut self) -> Result<Option<PathBuf>, FileError> {
        let (copy, next) = self.files.reload(&self.model)?;
        self.model = next;
        self.documents = documents(&self.files)?;
        Ok(Some(copy))
    }
}

impl<'a> DocumentSession<'a> {
    pub fn path(&self) -> &'a Path {
        self.path
    }
    pub fn model(&self) -> &'a Workbench {
        self.model
    }
    pub fn dirty(&self) -> bool {
        self.model.has_draft()
            || self.model.document().source() != self.baseline
            || self.recovery.pending()
    }
    pub fn recovery_error(&self) -> Option<String> {
        self.recovery.error()
    }
}

fn documents(files: &ProjectFiles) -> Result<Vec<ProjectDocument>, FileError> {
    files
        .paths
        .iter()
        .map(|path| {
            let key = DocumentKey::new(files.names.get(path).ok_or(FileError::Selection)?)
                .map_err(recite_writer_model::EditError::from)
                .map_err(recite_writer_model::WorkbenchError::from)?;
            Ok(ProjectDocument {
                path: path.clone(),
                key,
            })
        })
        .collect()
}

mod editing;
