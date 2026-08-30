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
            return super::uri_keyed_open_identity(uri);
        };
        let (canonical_path, path_exists) = super::canonical_or_normalized_path(&path);
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
