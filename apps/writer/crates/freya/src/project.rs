//! File ownership for the retained editor.
mod save;
use crate::recovery::{Recovery, RecoveryStore};
use recite_writer_model::{Document, ProjectContext, Workbench};

use std::{
    fs,
    path::{Path, PathBuf},
};

mod error;
pub use error::FileError;

pub struct ProjectFiles {
    pub paths: Vec<PathBuf>,
    pub current: PathBuf,
    saved: std::sync::Arc<str>,
    root: PathBuf,
    context: ProjectContext,
    search: std::sync::Arc<recite_writer_model::SearchIndex>,
    recovery: RecoveryStore,
    names: std::collections::BTreeMap<PathBuf, String>,
}

impl ProjectFiles {
    pub fn root(&self) -> &Path {
        &self.root
    }
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
        let saved = read_regular(&current)?.into();
        let recovery = RecoveryStore::open(&current)?;
        let context = crate::project_context::load(&report)?;
        let search =
            std::sync::Arc::new(recite_writer_model::SearchIndex::build(&context.documents));
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
            search,
            root,
        })
    }

    pub fn search_index(&self) -> std::sync::Arc<recite_writer_model::SearchIndex> {
        self.search.clone()
    }
    pub fn path_for_document(&self, name: &str) -> Option<PathBuf> {
        self.names
            .iter()
            .find(|(_, key)| key.as_str() == name)
            .map(|(path, _)| path.clone())
    }

    pub fn document_name(&self) -> Result<&str, FileError> {
        self.names
            .get(&self.current)
            .map(String::as_str)
            .ok_or(FileError::Selection)
    }

    pub fn dirty(&self, source: &str) -> bool {
        self.saved.as_ref() != source
    }

    pub fn workbench(&mut self) -> Result<Workbench, FileError> {
        let recovered = self.recovery.snapshot();
        let source = recovered.map_or(self.saved.as_ref(), |r| r.draft.source());
        let key = recite_core::DocumentKey::new(self.document_name()?)
            .map_err(recite_writer_model::EditError::from)
            .map_err(recite_writer_model::WorkbenchError::from)?;
        let document = Document::in_project(key, source, self.context.clone())
            .map_err(recite_writer_model::WorkbenchError::from)?;
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

    pub fn recovery_error(&self) -> Option<String> {
        self.recovery.error()
    }
    pub fn queue_checkpoint(&mut self, workbench: &Workbench) -> Result<(), FileError> {
        let recovery = (workbench.has_draft() || self.dirty(workbench.document().source()))
            .then(|| Recovery::new(self.saved.clone(), workbench.recovery()));
        self.recovery.queue(recovery)
    }
    pub fn select(&mut self, path: &Path) -> Result<Workbench, FileError> {
        self.select_at(path, |_| Ok(()))
    }

    pub fn select_at(
        &mut self,
        path: &Path,
        select: impl FnOnce(&mut Workbench) -> Result<(), recite_writer_model::WorkbenchError>,
    ) -> Result<Workbench, FileError> {
        if path == self.current {
            let mut workbench = self.workbench()?;
            select(&mut workbench)?;
            return Ok(workbench);
        }
        if !self.paths.iter().any(|candidate| candidate == path) {
            return Err(FileError::Selection);
        }
        let saved = read_regular(path)?.into();
        let recovery = RecoveryStore::open(path)?;
        let mut next = Self {
            paths: self.paths.clone(),
            current: path.to_owned(),
            saved,
            root: self.root.clone(),
            context: self.context.clone(),
            recovery,
            names: self.names.clone(),
            search: self.search.clone(),
        };
        next.update_saved_context()?;
        let mut workbench = next.workbench()?;
        select(&mut workbench)?;
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
            .map_err(recite_writer_model::EditError::from)
            .map_err(recite_writer_model::WorkbenchError::from)?;
        let document = Document::in_project(key, disk.clone(), context.clone())
            .map_err(recite_writer_model::WorkbenchError::from)?;
        let next = Workbench::from_document(document)?;
        let path = self.export(workbench)?;
        self.recovery.persist(None)?;
        self.saved = disk.into();
        self.search =
            std::sync::Arc::new(recite_writer_model::SearchIndex::build(&context.documents));
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
        self.search =
            std::sync::Arc::new(recite_writer_model::SearchIndex::build(&context.documents));
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
