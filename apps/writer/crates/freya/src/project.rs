//! File ownership for the retained editor.
mod external;
mod manifest;
mod recovery;
mod rename;
pub(crate) mod save;
mod sessions;
use crate::recovery::{Recovery, RecoveryStore};
use recite_compiler::authoring::WorkControl;
use recite_writer_model::{Document, ProjectContext, Workbench};

use std::{
    fs,
    path::{Path, PathBuf},
};

mod error;
pub use error::FileError;

pub struct ProjectFiles {
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
    pub fn root(&self) -> &Path {
        &self.root
    }
    pub(crate) fn open_with_control(
        path: &Path,
        control: &dyn WorkControl,
    ) -> Result<Self, FileError> {
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
        let saved = read_regular(&current)?.into();
        let recovery = RecoveryStore::open(&current)?;
        let context = crate::project_context::load(&report)?;
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
        let recovery = (workbench.has_draft()
            || self.saved.as_ref() != workbench.document().source())
        .then(|| Recovery::new(self.saved.clone(), workbench.recovery()));
        self.recovery.persist(recovery)
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
        let recovery = (workbench.has_draft()
            || self.saved.as_ref() != workbench.document().source())
        .then(|| Recovery::new(self.saved.clone(), workbench.recovery()));
        self.recovery.queue(recovery)
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
        let document =
            Document::in_project(key, disk.clone(), self.retained_context(context.clone()))
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
        let context = crate::project_context::load(&report)?;
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
