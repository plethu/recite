//! Retained file owners for a project; navigation never applies or discards a draft.
use crate::{DocumentSession, FileError, load_context};
use recite_compiler::authoring::SavedDocument;
use recite_core::DocumentKey;
use recite_writer_model::ProjectContext;
use std::{
    collections::BTreeMap,
    path::{Path, PathBuf},
};

pub struct ProjectDocument {
    pub path: PathBuf,
    pub key: DocumentKey,
}

pub struct ProjectSession {
    root: PathBuf,
    documents: Vec<ProjectDocument>,
    context: ProjectContext,
    current: DocumentSession,
    retained: BTreeMap<PathBuf, DocumentSession>,
}

impl ProjectSession {
    pub fn open(project: &Path, requested: Option<&Path>) -> Result<Self, FileError> {
        let report = recite_config::discover_project(project)?;
        let context = checked_context(&report)?;
        let root = report.manifest().project_root().to_owned();
        let requested = requested.map(|path| root.join(path));
        let documents = documents(&report);
        let document = documents
            .iter()
            .find(|document| requested.as_ref().is_none_or(|path| path == &document.path))
            .ok_or(FileError::Selection)?;
        let current =
            DocumentSession::open(document.path.clone(), document.key.clone(), context.clone())?;
        Ok(Self {
            root,
            documents,
            context,
            current,
            retained: BTreeMap::new(),
        })
    }

    pub fn root(&self) -> &Path {
        &self.root
    }
    pub fn documents(&self) -> &[ProjectDocument] {
        &self.documents
    }
    pub fn current(&self) -> &DocumentSession {
        &self.current
    }
    pub fn current_mut(&mut self) -> &mut DocumentSession {
        &mut self.current
    }
    pub fn sessions(&self) -> impl Iterator<Item = &DocumentSession> {
        std::iter::once(&self.current).chain(self.retained.values())
    }
    pub fn dirty(&self) -> bool {
        self.sessions().any(DocumentSession::dirty)
    }

    fn overlay(&self, mut context: ProjectContext, except: Option<&Path>) -> ProjectContext {
        let open = self
            .sessions()
            .filter(|session| Some(session.path()) != except)
            .map(|session| (session.model().document().key(), session.model().document()))
            .collect::<BTreeMap<_, _>>();
        for slot in &mut context.documents {
            if let Some(document) = open.get(slot.key()) {
                *slot =
                    SavedDocument::from_shared(document.key().clone(), document.source_snapshot());
            }
        }
        context
    }

    pub fn switch(&mut self, path: &Path) -> Result<(), FileError> {
        if path == self.current.path() {
            return Ok(());
        }
        let key = self
            .documents
            .iter()
            .find(|document| document.path == path)
            .ok_or(FileError::Selection)?
            .key
            .clone();
        let context = self.overlay(self.context.clone(), None);
        // Prepare a cold owner before touching the live one. A lock or read failure
        // must leave the active file, field draft and recovery ownership intact.
        let next = if self.retained.contains_key(path) {
            None
        } else {
            Some(DocumentSession::open_sibling(
                path.to_owned(),
                key,
                context.clone(),
                self.current.model().document(),
            )?)
        };
        self.current.park()?;
        let mut next = match next {
            Some(session) => session,
            None => {
                let session = self.retained.get_mut(path).ok_or(FileError::Selection)?;
                session.refresh(context)?;
                self.retained.remove(path).ok_or(FileError::Selection)?
            }
        };
        std::mem::swap(&mut self.current, &mut next);
        self.retained.insert(next.path().to_owned(), next);
        Ok(())
    }

    /// All owners retain their leases until the window is actually closed.
    pub fn checkpoint(&mut self) -> Result<(), FileError> {
        self.current.park()?;
        for session in self.retained.values_mut() {
            session.park()?;
        }
        Ok(())
    }

    /// Files save independently. On failure every remaining draft stays open.
    pub fn save_all(&mut self) -> Result<(), FileError> {
        for session in std::iter::once(&mut self.current).chain(self.retained.values_mut()) {
            if session.dirty() {
                session.save().map_err(|error| FileError::Saving {
                    path: session.path().to_owned(),
                    error: Box::new(error),
                })?;
            }
        }
        self.checkpoint()
    }

    pub fn reload_current(&mut self) -> Result<Option<PathBuf>, FileError> {
        let report = recite_config::discover_project(&self.root)?;
        let context = checked_context(&report)?;
        let documents = documents(&report);
        for session in self.sessions() {
            if !documents
                .iter()
                .any(|document| document.path == session.path())
            {
                return Err(FileError::SessionExcluded(session.path().to_owned()));
            }
        }
        let refreshed = self.overlay(context.clone(), Some(self.current.path()));
        let copy = self.current.reload(refreshed)?;
        self.context = context;
        self.documents = documents;
        Ok(copy)
    }
}

fn checked_context(
    report: &recite_config::ProjectDiscoveryReport,
) -> Result<ProjectContext, FileError> {
    if !report.is_complete() {
        return Err(FileError::Incomplete);
    }
    load_context(report)
}
fn documents(report: &recite_config::ProjectDiscoveryReport) -> Vec<ProjectDocument> {
    report
        .documents()
        .iter()
        .map(|document| ProjectDocument {
            path: document.path().to_owned(),
            key: document.key().clone(),
        })
        .collect()
}
