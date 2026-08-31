use std::collections::BTreeSet;

use lsp_types::Uri;

use super::schema_index::SchemaIndex;
use super::{DiagnosticRefresh, LspWorkspace};

impl LspWorkspace {
    pub(crate) fn save(&mut self, uri: Uri) -> Vec<DiagnosticRefresh> {
        let mut refreshes = Vec::new();
        if crate::paths::uri_to_file_path(&uri)
            .is_some_and(|path| self.saved.is_manifest_candidate(&path))
        {
            refreshes.extend(self.refresh_project_manifest());
        }
        let mut saved = self.saved.clone();
        let mut documents = self.documents.clone();
        let touched_saved = saved.refresh_uri(&uri);
        let open_identity_changed = self.refresh_open_identities(&saved, &mut documents);
        if (touched_saved || open_identity_changed)
            && self.rebuild_for_documents(saved, documents).is_err()
        {
            return refreshes;
        }
        if let Some(document) = self.documents.document(&uri) {
            refreshes.push(self.publish_open_document(document));
            return refreshes;
        }
        if let Some(refresh) = self
            .saved
            .document_by_uri(&uri)
            .map(|document| self.publish_saved_document(document))
        {
            refreshes.push(refresh);
        } else if touched_saved {
            refreshes.push(DiagnosticRefresh::Clear {
                uri,
                generation: self.generation,
            });
        }
        refreshes
    }

    pub(crate) fn refresh_project_manifest(&mut self) -> Vec<DiagnosticRefresh> {
        let old_document_uris = self.saved.document_uris().cloned().collect::<Vec<_>>();
        let old_uri = self
            .saved
            .manifest_path()
            .and_then(crate::paths::file_path_to_uri);
        let old_had_diagnostics = !self.saved.diagnostics().is_empty();
        let old_schema = self.schema.clone();
        let mut saved = self.saved.clone();
        let manifest_schema_path = saved.refresh_manifest();
        let schema = self
            .schema_override_path
            .clone()
            .map(|path| SchemaIndex::load(Some(path)))
            .unwrap_or_else(|| SchemaIndex::load(manifest_schema_path));
        let mut documents = self.documents.clone();
        self.refresh_open_identities(&saved, &mut documents);
        if self
            .rebuild_for_documents_with_schema(saved, documents, schema)
            .is_err()
        {
            return Vec::new();
        }
        let mut refreshes = Vec::new();
        if let Some(refresh) = self.project_diagnostics() {
            refreshes.push(refresh);
        } else if old_had_diagnostics && let Some(uri) = old_uri {
            refreshes.push(DiagnosticRefresh::Clear {
                uri,
                generation: self.generation,
            });
        }
        let schema_target_changed = old_schema.configured_path() != self.schema.configured_path();
        if schema_target_changed && let Some(refresh) = old_schema.clear_refresh(self.generation) {
            refreshes.push(refresh);
        }
        if (self.schema.needs_refresh() || (!schema_target_changed && old_schema.needs_refresh()))
            && let Some(refresh) = self.schema.refresh_or_clear(self.generation)
        {
            refreshes.push(refresh);
        }
        for uri in old_document_uris {
            if self.saved.document_by_uri(&uri).is_none() && self.documents.document(&uri).is_none()
            {
                refreshes.push(DiagnosticRefresh::Clear {
                    uri,
                    generation: self.generation,
                });
            }
        }
        refreshes
    }

    pub(crate) fn refresh_watched_uri(&mut self, uri: &Uri) -> Vec<DiagnosticRefresh> {
        if crate::paths::uri_to_file_path(uri)
            .is_some_and(|path| self.saved.is_manifest_candidate(&path))
        {
            return self.refresh_project_manifest();
        }
        if self.schema.matches_uri(uri)
            && let Some(refresh) = self.save_schema(uri)
        {
            return vec![refresh];
        }
        let mut saved = self.saved.clone();
        let mut documents = self.documents.clone();
        let touched_saved = saved.refresh_uri(uri);
        let open_identity_changed = self.refresh_open_identities(&saved, &mut documents);
        if touched_saved || open_identity_changed {
            let watched_keys = self.watched_document_keys(uri, &saved, &documents);
            if self.rebuild_for_documents(saved, documents).is_err() {
                return Vec::new();
            }
            if let Some(document) = watched_keys
                .iter()
                .find_map(|key| self.effective_open_document_for_key(key))
            {
                return vec![self.publish_open_document(document)];
            }
            return self
                .saved
                .document_by_uri(uri)
                .map(|document| vec![self.publish_saved_document(document)])
                .unwrap_or_else(|| {
                    vec![DiagnosticRefresh::Clear {
                        uri: uri.clone(),
                        generation: self.generation,
                    }]
                });
        }
        Vec::new()
    }

    fn watched_document_keys(
        &self,
        uri: &Uri,
        saved: &super::project_index::SavedProjectIndex,
        documents: &crate::documents::OpenDocumentStore,
    ) -> BTreeSet<recite_core::DocumentKey> {
        documents
            .document(uri)
            .and_then(super::document_key_for_open)
            .into_iter()
            .chain(
                saved
                    .document_by_uri(uri)
                    .and_then(super::document_key_for_saved),
            )
            .collect()
    }

    pub(crate) fn close(&mut self, uri: Uri) -> Vec<DiagnosticRefresh> {
        let mut documents = self.documents.clone();
        let Some(closed) = documents.close(&uri) else {
            return Vec::new();
        };
        let closed_key = super::document_key_for_open(&closed);
        let mut saved = self.saved.clone();
        saved.refresh_uri(&uri);
        self.refresh_open_identities(&saved, &mut documents);
        if self.rebuild_for_documents(saved, documents).is_err() {
            return Vec::new();
        }
        if self.schema.matches_uri(&uri) {
            let mut refreshes = vec![DiagnosticRefresh::Clear {
                uri: uri.clone(),
                generation: self.generation,
            }];
            if let Some(refresh) = self.schema.refresh_or_clear(self.generation) {
                let targets_closed_uri = match &refresh {
                    DiagnosticRefresh::Publish(diagnostics) => diagnostics.uri == uri,
                    DiagnosticRefresh::Clear {
                        uri: refresh_uri, ..
                    } => *refresh_uri == uri,
                };
                if !targets_closed_uri {
                    refreshes.push(refresh);
                }
            }
            return refreshes;
        }
        let remaining_open = closed_key
            .as_ref()
            .and_then(|key| self.effective_open_document_for_key(key))
            .or_else(|| {
                self.documents
                    .documents()
                    .find(|document| document.identity().saved_path == closed.identity().saved_path)
            });
        if let Some(document) = remaining_open {
            return vec![self.publish_open_document(document)];
        }
        vec![
            self.saved
                .document_by_uri(&uri)
                .map(|document| self.publish_saved_document(document))
                .unwrap_or(DiagnosticRefresh::Clear {
                    uri,
                    generation: self.generation,
                }),
        ]
    }
}
