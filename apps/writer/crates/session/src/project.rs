//! File ownership for the retained editor.
mod external;
pub mod manifest;
mod recovery;
mod rename;
pub mod save;
mod search;
mod sessions;
use crate::recovery::{Recovery, RecoveryStore};
use recite_compiler::authoring::{CancellationToken, WorkControl};
use recite_writer_model::{Document, ProjectContext, Workbench};
pub use sessions::checkpoint::{Checkpoint, ReadyCheckpoint};
pub use sessions::preparing::{PreparedSwitch, SwitchRequest};

use std::path::{Path, PathBuf};

pub use crate::{FileError, read_regular};

pub struct ProjectFiles {
    pub saving: bool,
    manifest: manifest::ManifestDraft,
    pub watch: Result<crate::external::Watch, String>,
    pub handoff: Option<crate::external::Handoff>,
    pub external: Option<external::ExternalComparison>,
    pub rename_review: Option<rename::RenameReview>,
    rename_undo: Vec<rename::RenameUndo>,
    rename_redo: Vec<rename::RenameUndo>,
    pub builds: crate::builds::Builds,
    pub declarations: Option<crate::declarations::Session>,
    retained: std::collections::BTreeMap<PathBuf, sessions::Retained>,
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
    pub fn open(path: &std::path::Path) -> Result<Self, FileError> {
        Self::open_with_control(path, &CancellationToken::new())
    }

    pub fn workbench(&mut self) -> Result<recite_writer_model::Workbench, FileError> {
        self.workbench_with_control(None, &CancellationToken::new())
    }
}

impl ProjectFiles {
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn open_with_control(path: &Path, control: &dyn WorkControl) -> Result<Self, FileError> {
        control.checkpoint()?;
        let report = recite_config::discover_project(path)?;
        control.checkpoint()?;
        if !report.is_complete() {
            return Err(FileError::Incomplete);
        }
        let paths: Vec<_> = report
            .documents()
            .iter()
            .map(|d| d.path().to_owned())
            .collect();
        let current = paths.first().ok_or(FileError::Empty)?.clone();
        let saved: std::sync::Arc<str> = read_regular(&current)?.into();
        let recovery = RecoveryStore::open(&current)?;
        let mut context = crate::load_context(&report)?;
        if let Some(document) = report.documents().first() {
            crate::share_saved_source(&mut context, document.key(), &saved);
        }
        control.checkpoint()?;
        let search = std::sync::Arc::new(recite_writer_model::SearchIndex::build_with_control(
            &context.documents,
            control,
        )?);
        let root = report.manifest().project_root().to_owned();
        let names = report
            .documents()
            .iter()
            .map(|d| (d.path().to_owned(), d.key().as_str().to_owned()))
            .collect();
        let manifest = manifest::ManifestDraft::open(&root)?;
        let watch = crate::external::Watch::new(&root);
        control.checkpoint()?;
        Ok(Self {
            saving: false,
            manifest,
            watch,
            external: None,
            handoff: None,
            builds: Default::default(),
            rename_review: None,
            rename_undo: Vec::new(),
            rename_redo: Vec::new(),
            declarations: None,
            retained: Default::default(),
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
        self.saved.as_ref() != source || self.recovery.pending()
    }

    pub fn has_recovery(&self) -> bool {
        self.recovery.snapshot().is_some() || self.manifest.dirty()
    }

    pub fn checkpoint(&mut self, workbench: &Workbench) -> Result<(), FileError> {
        self.recovery.persist(self.recovery_value(workbench))
    }
    fn recovery_value(&self, workbench: &Workbench) -> Option<Recovery> {
        (workbench.has_draft() || self.saved.as_ref() != workbench.document().source())
            .then(|| Recovery::new(self.saved.clone(), workbench.recovery()))
    }

    pub fn recovery_error(&self) -> Option<String> {
        self.recovery.error().or_else(|| {
            self.declarations
                .as_ref()
                .and_then(|s| s.source.as_ref())
                .and_then(|s| s.recovery_error())
        })
    }
    pub fn queue_checkpoint(&mut self, workbench: &Workbench) -> Result<(), FileError> {
        self.recovery.queue(self.recovery_value(workbench))
    }

    /// Preserve the complete local session before accepting the disk version.
    pub fn reload(&mut self, workbench: &Workbench) -> Result<(PathBuf, Workbench), FileError> {
        self.require_idle_save()?;
        let report = recite_config::discover_project(&self.root)?;
        if !report.is_complete() {
            return Err(FileError::Incomplete);
        }
        let context = crate::load_context(&report)?;
        let disk = read_regular(&self.current)?;
        let key = recite_core::DocumentKey::new(self.document_name()?)
            .map_err(recite_writer_model::EditError::from)
            .map_err(recite_writer_model::WorkbenchError::from)?;
        let document =
            Document::in_project(key, disk.clone(), self.retained_context(context.clone()))
                .map_err(recite_writer_model::WorkbenchError::from)?;
        let next = Workbench::from_document(document, None)?;
        let path = self.export(workbench)?;
        self.recovery.persist(None)?;
        self.saved = disk.into();
        self.search = self.refreshed_search(&context.documents);
        self.context = context;
        Ok((path, next))
    }

    pub fn refresh(&mut self, workbench: &mut Workbench) -> Result<(), FileError> {
        self.require_idle_save()?;
        let report = recite_config::discover_project(&self.root)?;
        if !report.is_complete() {
            return Err(FileError::Incomplete);
        }
        for path in self.session_paths() {
            if !report.documents().iter().any(|d| d.path() == path) {
                return Err(FileError::SessionExcluded(path.clone()));
            }
        }
        let schema_changed = self.declarations.as_ref().is_some_and(|session| {
            !session.matches_schema(
                report
                    .manifest()
                    .source()
                    .manifest()
                    .project
                    .schema
                    .as_deref(),
            )
        });
        if schema_changed
            && self
                .declarations
                .as_ref()
                .is_some_and(|session| session.dirty() || session.busy())
        {
            return Err(FileError::SchemaSessionActive);
        }
        let context = crate::load_context(&report)?;
        self.manifest
            .refresh(report.manifest().source().source_text());
        workbench.refresh_project(self.retained_context(context.clone()))?;
        if schema_changed {
            self.declarations = None;
        }
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
        self.search = self.refreshed_search(&context.documents);
        self.context = context;
        Ok(())
    }

    pub fn export(&self, workbench: &Workbench) -> Result<PathBuf, FileError> {
        crate::export_recovery(
            &self.current,
            &Recovery::new(self.saved.clone(), workbench.recovery()),
        )
    }
}

#[cfg(test)]
mod tests;
