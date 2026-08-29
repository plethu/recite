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
            refreshes.push(DiagnosticRefresh::publish_open(
                &open_refresh.document,
                self.generation,
            ));
            return refreshes;
        }
        if let Some(refresh) = self
            .saved
            .document_by_uri(&uri)
            .map(|document| DiagnosticRefresh::publish_saved(document, self.generation))
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
        if self.saved.refresh_uri(uri) {
            self.rebuild_next_generation();
            return self
                .saved
                .document_by_uri(uri)
                .map(|document| vec![DiagnosticRefresh::publish_saved(document, self.generation)])
                .unwrap_or_else(|| {
                    vec![DiagnosticRefresh::Clear {
                        uri: uri.clone(),
                        generation: self.generation,
                    }]
                });
        }
        Vec::new()
    }

    pub(crate) fn close(&mut self, uri: Uri) -> Option<DiagnosticRefresh> {
        self.documents.close(&uri)?;
        self.saved.refresh_uri(&uri);
        self.rebuild_next_generation();
        Some(
            self.saved
                .document_by_uri(&uri)
                .map(|document| DiagnosticRefresh::publish_saved(document, self.generation))
                .unwrap_or(DiagnosticRefresh::Clear {
                    uri,
                    generation: self.generation,
                }),
        )
    }
}
