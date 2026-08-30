use std::collections::BTreeSet;

use lsp_types::Uri;

use super::{DiagnosticRefresh, LspWorkspace};

impl LspWorkspace {
    pub(crate) fn save(&mut self, uri: Uri) -> Vec<DiagnosticRefresh> {
        let mut refreshes = Vec::new();
        if crate::paths::uri_to_file_path(&uri)
            .is_some_and(|path| self.saved.is_manifest_candidate(&path))
        {
            refreshes.extend(self.refresh_project_manifest());
        }
        let touched_saved = self.saved.refresh_uri(&uri);
        let open_identity = self.open_identity(uri.clone());
        let open_refresh = self.documents.refresh_identity(open_identity);
        if touched_saved
            || open_refresh
                .as_ref()
                .is_some_and(|refresh| refresh.identity_changed)
        {
            self.rebuild_next_generation();
        }
        if let Some(open_refresh) = open_refresh {
            refreshes.push(self.publish_open_document(&open_refresh.document));
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
        self.saved.refresh_manifest();
        self.rebuild_next_generation();
        let mut refreshes = Vec::new();
        if let Some(refresh) = self.project_diagnostics() {
            refreshes.push(refresh);
        } else if old_had_diagnostics && let Some(uri) = old_uri {
            refreshes.push(DiagnosticRefresh::Clear {
                uri,
                generation: self.generation,
            });
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
        let watched_keys = self.watched_document_keys(uri);
        if self.saved.refresh_uri(uri) {
            self.rebuild_next_generation();
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

    fn watched_document_keys(&self, uri: &Uri) -> BTreeSet<recite_core::DocumentKey> {
        self.documents
            .document(uri)
            .and_then(super::document_key_for_open)
            .into_iter()
            .chain(
                self.saved
                    .document_by_uri(uri)
                    .and_then(super::document_key_for_saved),
            )
            .collect()
    }

    pub(crate) fn close(&mut self, uri: Uri) -> Option<DiagnosticRefresh> {
        let closed = self.documents.close(&uri)?;
        let closed_key = super::document_key_for_open(&closed);
        self.saved.refresh_uri(&uri);
        self.rebuild_next_generation();
        if let Some(document) = closed_key
            .as_ref()
            .and_then(|key| self.effective_open_document_for_key(key))
        {
            return Some(self.publish_open_document(document));
        }
        Some(
            self.saved
                .document_by_uri(&uri)
                .map(|document| self.publish_saved_document(document))
                .unwrap_or(DiagnosticRefresh::Clear {
                    uri,
                    generation: self.generation,
                }),
        )
    }
}
