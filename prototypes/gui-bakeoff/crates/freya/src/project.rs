//! File ownership for the retained editor.
mod save;
use crate::recovery::{Recovery, RecoveryStore};
use recite_bakeoff_authoring::{Document, ProjectContext, Workbench};

use std::{
    fs, io,
    path::{Path, PathBuf},
};

#[derive(Debug, thiserror::Error)]
pub enum FileError {
    #[error(transparent)]
    Discovery(#[from] recite_config::ProjectDiscoveryError),
    #[error(
        "Project discovery is incomplete. Repair the project's discovery diagnostics before opening it here."
    )]
    Incomplete,
    #[error("Project validation failed: {}", crate::project_context::diagnostic_messages(.0))]
    Validation(Vec<recite_core::Diagnostic>),
    #[error(
        "This document is already open in another writer. Close it there before opening it here."
    )]
    RecoveryInUse,
    #[error("The recovery snapshot uses an unsupported version; it has been left untouched.")]
    RecoveryVersion,
    #[error("Recovery snapshot could not be read or written: {0}")]
    Recovery(#[from] serde_json::Error),
    #[error(transparent)]
    Workbench(#[from] recite_bakeoff_authoring::WorkbenchError),
    #[error("The project has no Recite source files.")]
    Empty,
    #[error("The selected file is no longer in this project.")]
    Selection,
    #[error(
        "This file changed on disk. Your edits remain open. Use Keep recovery copy and reload disk to retain both versions."
    )]
    Conflict,
    #[error("Refusing to replace a symbolic link or a non-regular file.")]
    FileKind,
    #[error(
        "Could not acquire the editor's save lock: {0}. A crashed editor may have left a .recite-editor.lock file beside the source."
    )]
    Locked(io::Error),
    #[error("File operation failed: {0}")]
    Io(#[from] io::Error),
}

pub struct ProjectFiles {
    pub paths: Vec<PathBuf>,
    pub current: PathBuf,
    saved: String,
    root: PathBuf,
    context: ProjectContext,
    recovery: RecoveryStore,
    names: std::collections::BTreeMap<PathBuf, String>,
}

impl ProjectFiles {
    pub fn open(path: &Path) -> Result<Self, FileError> {
        Self::open_at(path, None)
    }

    fn open_at(path: &Path, selection: Option<&Path>) -> Result<Self, FileError> {
        let report = recite_config::discover_project(path)?;
        if !report.is_complete() {
            return Err(FileError::Incomplete);
        }
        let paths: Vec<_> = report
            .documents()
            .iter()
            .map(|d| d.path().to_owned())
            .collect();
        let current = match selection {
            Some(path) if paths.iter().any(|p| p == path) => path.to_owned(),
            Some(_) => return Err(FileError::Selection),
            None => paths.first().ok_or(FileError::Empty)?.clone(),
        };
        let saved = read_regular(&current)?;
        let recovery = RecoveryStore::open(&current)?;
        let context = crate::project_context::load(&report)?;
        let root = report.manifest().project_root().to_owned();
        let names = report
            .documents()
            .iter()
            .map(|d| (d.path().to_owned(), d.key().as_str().to_owned()))
            .collect();
        Ok(Self {
            paths,
            current,
            saved,
            names,
            recovery,
            context,
            root,
        })
    }

    pub fn document_name(&self) -> Result<&str, FileError> {
        self.names
            .get(&self.current)
            .map(String::as_str)
            .ok_or(FileError::Selection)
    }

    pub fn dirty(&self, source: &str) -> bool {
        self.saved != source
    }

    pub fn workbench(&mut self) -> Result<Workbench, FileError> {
        let recovered = self.recovery.snapshot();
        let source = recovered.map_or(self.saved.as_str(), |r| r.draft.source());
        let key = recite_core::DocumentKey::new(self.document_name()?)
            .map_err(recite_bakeoff_authoring::EditError::from)
            .map_err(recite_bakeoff_authoring::WorkbenchError::from)?;
        let document = Document::in_project(key, source, self.context.clone())
            .map_err(recite_bakeoff_authoring::WorkbenchError::from)?;
        let mut workbench = Workbench::from_document(document)?;
        if let Some(recovery) = recovered {
            recovery.draft.restore(&mut workbench)?;
            self.saved = recovery.baseline.clone();
        }
        Ok(workbench)
    }

    pub fn has_recovery(&self) -> bool {
        self.recovery.snapshot().is_some()
    }

    pub fn checkpoint(&mut self, workbench: &Workbench) -> Result<(), FileError> {
        let recovery = (workbench.has_draft() || self.dirty(workbench.document().source()))
            .then(|| Recovery::new(self.saved.clone(), workbench.recovery()));
        self.recovery.persist(recovery)
    }

    pub fn select(&mut self, path: &Path) -> Result<Workbench, FileError> {
        if path == self.current {
            return self.workbench();
        }
        let mut next = Self::open_at(&self.root, Some(path))?;
        let workbench = next.workbench()?;
        *self = next;
        Ok(workbench)
    }

    /// Preserve the complete local session before accepting the disk version.
    pub fn reload(&mut self, workbench: &Workbench) -> Result<(PathBuf, Workbench), FileError> {
        let report = recite_config::discover_project(&self.root)?;
        if !report.is_complete() {
            return Err(FileError::Incomplete);
        }
        let context = crate::project_context::load(&report)?;
        let disk = read_regular(&self.current)?;
        let key = recite_core::DocumentKey::new(self.document_name()?)
            .map_err(recite_bakeoff_authoring::EditError::from)
            .map_err(recite_bakeoff_authoring::WorkbenchError::from)?;
        let document = Document::in_project(key, disk.clone(), context.clone())
            .map_err(recite_bakeoff_authoring::WorkbenchError::from)?;
        let next = Workbench::from_document(document)?;
        let path = self.export(workbench)?;
        self.recovery.persist(None)?;
        self.saved = disk;
        self.context = context;
        Ok((path, next))
    }

    pub fn refresh(&mut self, workbench: &mut Workbench) -> Result<(), FileError> {
        let report = recite_config::discover_project(&self.root)?;
        if !report.is_complete() {
            return Err(FileError::Incomplete);
        }
        if !report.documents().iter().any(|d| d.path() == self.current) {
            return Err(FileError::Selection);
        }
        let context = crate::project_context::load(&report)?;
        workbench.refresh_project(context.clone())?;
        self.paths = report
            .documents()
            .iter()
            .map(|d| d.path().to_owned())
            .collect();
        self.names = report
            .documents()
            .iter()
            .map(|d| (d.path().to_owned(), d.key().as_str().to_owned()))
            .collect();
        self.context = context;
        Ok(())
    }

    pub fn export(&self, workbench: &Workbench) -> Result<PathBuf, FileError> {
        let parent = self.current.parent().ok_or(FileError::Selection)?;
        let mut copy = tempfile::Builder::new()
            .prefix(".recite-recovered-")
            .suffix(".json")
            .tempfile_in(parent)?;
        serde_json::to_writer_pretty(
            &mut copy,
            &Recovery::new(self.saved.clone(), workbench.recovery()),
        )?;
        copy.as_file().sync_all()?;
        let (_, path) = copy.keep().map_err(|error| error.error)?;
        #[cfg(unix)]
        fs::File::open(parent)?.sync_all()?;
        Ok(path)
    }
}

pub(super) fn read_regular(path: &Path) -> Result<String, FileError> {
    let metadata = fs::symlink_metadata(path)?;
    if !metadata.file_type().is_file() {
        return Err(FileError::FileKind);
    }
    Ok(fs::read_to_string(path)?)
}

#[cfg(test)]
mod tests;
