use std::fs;
use std::path::{Component, Path, PathBuf};

use lsp_types::{TextDocumentContentChangeEvent, Uri};

use super::project_index::SavedProjectIndex;
use super::{DiagnosticRefresh, LspWorkspace, WorkspaceChangeResult};
use crate::documents::{DocumentChangeResult, OpenDocumentStore};
use crate::paths::uri_to_file_path;
use crate::summary::OpenFileIdentity;

impl LspWorkspace {
    pub(crate) fn open(
        &mut self,
        uri: Uri,
        version: i32,
        text: String,
    ) -> Option<DiagnosticRefresh> {
        if self.documents.document(&uri).is_some() {
            return None;
        }
        let identity = self.open_identity(uri.clone());
        let mut documents = self.documents.clone();
        documents.open(identity, version, text);
        self.rebuild_state(self.saved.clone(), documents).ok()?;
        self.documents
            .document(&uri)
            .map(|document| self.publish_open_document(document))
    }

    pub(crate) fn change(
        &mut self,
        uri: Uri,
        version: i32,
        changes: Vec<TextDocumentContentChangeEvent>,
    ) -> WorkspaceChangeResult {
        let identity = self.open_identity(uri.clone());
        let mut documents = self.documents.clone();
        match documents.change(identity, version, changes) {
            DocumentChangeResult::Accepted(_) => {
                if self.rebuild_state(self.saved.clone(), documents).is_err() {
                    return WorkspaceChangeResult::Rejected;
                }
                let Some(document) = self.documents.document(&uri) else {
                    return WorkspaceChangeResult::Rejected;
                };
                WorkspaceChangeResult::Accepted(self.publish_open_document(document))
            }
            DocumentChangeResult::Stale => WorkspaceChangeResult::Stale,
            DocumentChangeResult::Malformed => WorkspaceChangeResult::Malformed,
            DocumentChangeResult::Unopened => WorkspaceChangeResult::Unopened,
        }
    }

    pub(super) fn refresh_open_identities(
        &self,
        saved: &SavedProjectIndex,
        documents: &mut OpenDocumentStore,
    ) -> bool {
        let uris = documents
            .documents()
            .map(|document| document.identity().uri.clone())
            .collect::<Vec<_>>();
        let mut changed = false;
        for uri in uris {
            changed |= documents
                .refresh_identity(self.open_identity_for(saved, uri))
                .is_some_and(|refresh| refresh.identity_changed);
        }
        changed
    }

    fn open_identity(&self, uri: Uri) -> OpenFileIdentity {
        self.open_identity_for(&self.saved, uri)
    }

    fn open_identity_for(&self, saved: &SavedProjectIndex, uri: Uri) -> OpenFileIdentity {
        let Some(path) = uri_to_file_path(&uri) else {
            return uri_keyed_open_identity(uri);
        };
        let (canonical_path, path_exists) = canonical_or_normalized_path(&path);
        let project_relative_path = path_exists
            .then(|| saved.project_key_for_path(&canonical_path))
            .flatten();

        OpenFileIdentity {
            uri,
            saved_path: Some(canonical_path),
            project_relative_path,
        }
    }
}

fn canonical_or_normalized_path(path: &Path) -> (PathBuf, bool) {
    if let Ok(canonical_path) = fs::canonicalize(path) {
        return (canonical_path, true);
    }

    let normalized = lexically_normalized_path(path);
    let mut missing_components = Vec::new();
    let mut cursor = normalized.as_path();
    loop {
        if let Ok(canonical_parent) = fs::canonicalize(cursor) {
            let mut path = canonical_parent;
            for component in missing_components.iter().rev() {
                path.push(component);
            }
            return (path, false);
        }

        let Some(component) = cursor.file_name() else {
            return (normalized, false);
        };
        missing_components.push(component.to_owned());
        let Some(parent) = cursor.parent() else {
            return (normalized, false);
        };
        cursor = parent;
    }
}

fn lexically_normalized_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(Path::new(std::path::MAIN_SEPARATOR_STR)),
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(component) => normalized.push(component),
        }
    }
    normalized
}

fn uri_keyed_open_identity(uri: Uri) -> OpenFileIdentity {
    OpenFileIdentity {
        uri,
        saved_path: None,
        project_relative_path: None,
    }
}
