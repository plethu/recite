//! Disk reload prepares independently, archives first, and adopts through the live lease.
use super::{FileError, ProjectDocument, ProjectFiles};
use crate::recovery::{Recovery, worker::Durability};
use recite_compiler::authoring::{CancellationToken, SavedDocument, WorkControl};
use recite_writer_model::{
    Document, EditError, NavigationStamp, ProjectContext, SearchIndex, View, Workbench,
    WorkbenchError,
};
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
    sync::Arc,
};

struct RetainedOrigin {
    path: PathBuf,
    selection: NavigationStamp,
    baseline: Arc<str>,
}
struct Origin {
    root: PathBuf,
    path: PathBuf,
    selection: NavigationStamp,
    baseline: Arc<str>,
    context: Arc<ProjectContext>,
    retained: Vec<RetainedOrigin>,
}
pub struct ReloadRequest {
    origin: Origin,
    recovery: Recovery,
    view: View,
    overlays: Vec<SavedDocument>,
}
struct Snapshot {
    baseline: Arc<str>,
    context: Arc<ProjectContext>,
    search: Arc<SearchIndex>,
    paths: Vec<PathBuf>,
    names: BTreeMap<PathBuf, String>,
    model: Workbench,
    documents: Vec<ProjectDocument>,
}
/// Adoption swaps displaced payloads into this receipt for worker retirement.
pub struct PreparedReload {
    origin: Origin,
    archive: PathBuf,
    snapshot: Snapshot,
    cleanup: Option<Durability<Recovery>>,
}
impl ProjectFiles {
    pub fn reload_request(&self, model: &Workbench) -> Result<ReloadRequest, FileError> {
        self.require_idle_save()?;
        Ok(ReloadRequest {
            origin: Origin {
                root: self.root.clone(),
                path: self.current.clone(),
                selection: model.navigation_stamp(),
                baseline: self.saved.clone(),
                context: self.context.clone(),
                retained: self
                    .retained
                    .iter()
                    .map(|(path, session)| RetainedOrigin {
                        path: path.clone(),
                        selection: session.model.navigation_stamp(),
                        baseline: session.baseline.clone(),
                    })
                    .collect(),
            },
            recovery: Recovery::new(self.saved.clone(), model.recovery()),
            view: model.view().clone(),
            overlays: self
                .retained
                .values()
                .map(|session| {
                    SavedDocument::from_shared(
                        session.model.document().key().clone(),
                        session.model.document().source_snapshot(),
                    )
                })
                .collect(),
        })
    }
    pub(super) fn publish_reload(
        &mut self,
        model: &Workbench,
        prepared: &mut PreparedReload,
    ) -> Result<(), FileError> {
        self.require_idle_save()?;
        if !prepared.origin.is_current(self, model) {
            return Err(WorkbenchError::from(EditError::Stale).into());
        }
        // All validation and allocation precede this irreversible cleanup request.
        let cleanup = self.recovery.begin_persist(None)?;
        let next = &mut prepared.snapshot;
        std::mem::swap(&mut self.saved, &mut next.baseline);
        std::mem::swap(&mut self.context, &mut next.context);
        std::mem::swap(&mut self.search, &mut next.search);
        std::mem::swap(&mut self.paths, &mut next.paths);
        std::mem::swap(&mut self.names, &mut next.names);
        prepared.cleanup = Some(cleanup);
        Ok(())
    }
    /// Compatibility path for the synchronous frontend; native hosts use the request/receipt API.
    pub fn reload(&mut self, model: &Workbench) -> Result<(PathBuf, Workbench), FileError> {
        let mut prepared = self
            .reload_request(model)?
            .prepare(&CancellationToken::new())?;
        // Preserve this API's all-or-error behavior before returning the replacement model.
        self.recovery.persist(None)?;
        self.publish_reload(model, &mut prepared)?;
        Ok((prepared.archive, prepared.snapshot.model))
    }
}
impl Origin {
    fn is_current(&self, files: &ProjectFiles, model: &Workbench) -> bool {
        self.root == files.root
            && self.path == files.current
            && self.selection.is_current(model)
            && Arc::ptr_eq(&self.baseline, &files.saved)
            && Arc::ptr_eq(&self.context, &files.context)
            && self.retained.len() == files.retained.len()
            && self.retained.iter().all(|origin| {
                files.retained.get(&origin.path).is_some_and(|session| {
                    origin.selection.is_current(&session.model)
                        && Arc::ptr_eq(&origin.baseline, &session.baseline)
                })
            })
    }
}
impl ReloadRequest {
    pub fn prepare(self, control: &dyn WorkControl) -> Result<PreparedReload, FileError> {
        control.checkpoint()?;
        let report = recite_config::discover_project(&self.origin.root)?;
        control.checkpoint()?;
        if !report.is_complete() {
            return Err(FileError::Incomplete);
        }
        if report.manifest().project_root() != self.origin.root {
            return Err(FileError::ProjectRootChanged);
        }
        for path in std::iter::once(&self.origin.path)
            .chain(self.origin.retained.iter().map(|origin| &origin.path))
        {
            if !report
                .documents()
                .iter()
                .any(|document| document.path() == path)
            {
                return Err(FileError::SessionExcluded(path.clone()));
            }
        }
        let key = report
            .documents()
            .iter()
            .find(|document| document.path() == self.origin.path)
            .ok_or(FileError::Selection)?
            .key()
            .clone();
        let baseline: Arc<str> = crate::read_regular(&self.origin.path)?.into();
        let mut saved_context = crate::load_context(&report)?;
        crate::share_saved_source(&mut saved_context, &key, &baseline);
        let search = Arc::new(SearchIndex::build_with_control(
            &saved_context.documents,
            control,
        )?);
        let mut accepted_context = saved_context.clone();
        for saved in &mut accepted_context.documents {
            if let Some(overlay) = self
                .overlays
                .iter()
                .find(|overlay| overlay.key() == saved.key())
            {
                *saved = overlay.clone();
            }
        }
        let document =
            Document::in_project_with_control(key, baseline.to_string(), accepted_context, control)
                .map_err(WorkbenchError::from)?;
        let mut model = Workbench::from_document(document, None)?;
        if let View::Passage(id) = self.view
            && model
                .document()
                .passage_snapshot()
                .is_ok_and(|passages| passages.iter().any(|passage| passage.id == id))
        {
            model.select(View::Passage(id))?;
        }
        let documents = report
            .documents()
            .iter()
            .map(|document| ProjectDocument {
                path: document.path().to_owned(),
                key: document.key().clone(),
            })
            .collect::<Vec<_>>();
        let paths = documents
            .iter()
            .map(|document| document.path.clone())
            .collect();
        let names = documents
            .iter()
            .map(|document| (document.path.clone(), document.key.to_string()))
            .collect();
        control.checkpoint()?;
        // Once written, this archive is a durable user artifact even if adoption is cancelled.
        let archive = crate::export_recovery(&self.origin.path, &self.recovery)?;
        Ok(PreparedReload {
            origin: self.origin,
            archive,
            snapshot: Snapshot {
                baseline,
                context: Arc::new(saved_context),
                search,
                paths,
                names,
                model,
                documents,
            },
            cleanup: None,
        })
    }
}
impl PreparedReload {
    pub fn document(&self) -> &Path {
        &self.origin.path
    }
    pub fn archive(&self) -> &Path {
        &self.archive
    }
    pub(super) fn exchange(&mut self, model: &mut Workbench, documents: &mut Vec<ProjectDocument>) {
        std::mem::swap(model, &mut self.snapshot.model);
        std::mem::swap(documents, &mut self.snapshot.documents);
    }
    /// Run on a worker after adoption; failure reports cleanup, never rolls back accepted source.
    pub fn finish_cleanup(&mut self) -> Result<(), FileError> {
        self.cleanup.take().map_or(Ok(()), Durability::wait)
    }
}

#[cfg(test)]
mod tests;
