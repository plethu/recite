//! Discover and compile project context off-thread without reloading any live source buffer.
use super::{FileError, ProjectDocument, ProjectFiles};
use recite_compiler::authoring::{CancellationToken, SavedDocument, WorkControl};
use recite_writer_model::{
    CompletionSnapshot, ContextRefreshRequest, EditError, PreparedContextRefresh, ProjectContext,
    SearchIndex, Workbench, WorkbenchError,
};
use std::{collections::BTreeMap, path::PathBuf, sync::Arc};

struct Origin {
    root: PathBuf,
    current: PathBuf,
    context: Arc<ProjectContext>,
    retained: Vec<(PathBuf, CompletionSnapshot)>,
    configuration_change: Option<super::settings::ConfigurationChange>,
}
pub struct RefreshRequest {
    origin: Origin,
    analysis: ContextRefreshRequest,
    overlays: Vec<SavedDocument>,
    search: Arc<SearchIndex>,
}
pub struct PreparedRefresh {
    origin: Origin,
    analysis: PreparedContextRefresh,
    context: Arc<ProjectContext>,
    search: Arc<SearchIndex>,
    paths: Vec<PathBuf>,
    names: BTreeMap<PathBuf, String>,
    documents: Vec<ProjectDocument>,
    manifest: String,
    schema: Option<String>,
    retired_declarations: Option<crate::declarations::Session>,
}
impl ProjectFiles {
    pub fn refresh_request(&self, model: &Workbench) -> Result<RefreshRequest, FileError> {
        self.require_idle_save()?;
        Ok(RefreshRequest {
            origin: Origin {
                root: self.root.clone(),
                current: self.current.clone(),
                context: self.context.clone(),
                configuration_change: self.configuration_change.clone(),
                retained: self
                    .retained
                    .iter()
                    .map(|(path, session)| {
                        (path.clone(), session.model.document().completion_snapshot())
                    })
                    .collect(),
            },
            analysis: model.context_refresh_request(),
            search: self.search.clone(),
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
    pub fn accept_refresh(
        &mut self,
        model: &mut Workbench,
        prepared: &mut PreparedRefresh,
    ) -> Result<(), FileError> {
        self.require_idle_save()?;
        if !prepared.origin.is_current(self) {
            return Err(WorkbenchError::from(EditError::Stale).into());
        }
        let schema_changed = self
            .declarations
            .as_ref()
            .is_some_and(|session| !session.matches_schema(prepared.schema.as_deref()));
        if schema_changed
            && self
                .declarations
                .as_ref()
                .is_some_and(|session| session.dirty() || session.busy())
        {
            return Err(FileError::SchemaSessionActive);
        }
        model.accept_context_refresh(&mut prepared.analysis)?;
        self.manifest
            .refresh(std::mem::take(&mut prepared.manifest));
        if schema_changed {
            prepared.retired_declarations = self.declarations.take();
        }
        std::mem::swap(&mut self.context, &mut prepared.context);
        std::mem::swap(&mut self.search, &mut prepared.search);
        std::mem::swap(&mut self.paths, &mut prepared.paths);
        std::mem::swap(&mut self.names, &mut prepared.names);
        self.configuration_change = None;
        Ok(())
    }
    pub fn refresh(&mut self, model: &mut Workbench) -> Result<(), FileError> {
        let mut prepared = self
            .refresh_request(model)?
            .prepare(&CancellationToken::new())?;
        self.accept_refresh(model, &mut prepared)
    }
}
impl Origin {
    fn is_current(&self, files: &ProjectFiles) -> bool {
        self.root == files.root
            && match (&self.configuration_change, &files.configuration_change) {
                (None, None) => true,
                (Some(before), Some(current)) => before.matches(current),
                _ => false,
            }
            && self.current == files.current
            && Arc::ptr_eq(&self.context, &files.context)
            && self.retained.len() == files.retained.len()
            && self.retained.iter().all(|(path, analysis)| {
                files
                    .retained
                    .get(path)
                    .is_some_and(|session| analysis.is_current(session.model.document()))
            })
    }
}
impl RefreshRequest {
    pub fn prepare(self, control: &dyn WorkControl) -> Result<PreparedRefresh, FileError> {
        control.checkpoint()?;
        let report = recite_config::discover_project(&self.origin.root)?;
        control.checkpoint()?;
        if report.manifest().project_root() != self.origin.root {
            return Err(FileError::ProjectRootChanged);
        }
        if !report.is_complete() {
            return Err(FileError::Incomplete);
        }
        for path in std::iter::once(&self.origin.current)
            .chain(self.origin.retained.iter().map(|(path, _)| path))
        {
            if !report
                .documents()
                .iter()
                .any(|document| document.path() == path)
            {
                return Err(FileError::SessionExcluded(path.clone()));
            }
        }
        let context = crate::load_context(&report)?;
        let search = super::search::refresh_search(
            &self.origin.context.documents,
            &self.search,
            &context.documents,
            control,
        )?;
        let mut accepted = context.clone();
        for saved in &mut accepted.documents {
            if let Some(overlay) = self
                .overlays
                .iter()
                .find(|overlay| overlay.key() == saved.key())
            {
                *saved = overlay.clone();
            }
        }
        let analysis = self.analysis.prepare(accepted, control)?;
        let documents: Vec<_> = report
            .documents()
            .iter()
            .map(|document| ProjectDocument {
                path: document.path().to_owned(),
                key: document.key().clone(),
            })
            .collect();
        control.checkpoint()?;
        Ok(PreparedRefresh {
            origin: self.origin,
            analysis,
            context: context.into(),
            search,
            paths: documents
                .iter()
                .map(|document| document.path.clone())
                .collect(),
            names: documents
                .iter()
                .map(|document| (document.path.clone(), document.key.to_string()))
                .collect(),
            documents,
            manifest: report.manifest().source().source_text().to_owned(),
            schema: report.manifest().source().manifest().project.schema.clone(),
            retired_declarations: None,
        })
    }
}
impl PreparedRefresh {
    pub(super) fn exchange_documents(&mut self, documents: &mut Vec<ProjectDocument>) {
        std::mem::swap(documents, &mut self.documents);
    }
}

#[cfg(test)]
mod tests;
