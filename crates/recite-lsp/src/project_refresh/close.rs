use super::super::{DiagnosticRefresh, LspWorkspace};

impl LspWorkspace {
    pub(crate) fn close(&mut self, uri: lsp_types::Uri) -> Vec<DiagnosticRefresh> {
        let mut documents = self.documents.clone();
        let Some(closed) = documents.close(&uri) else {
            return Vec::new();
        };
        let closed_key = super::super::document_key_for_open(&closed);
        let mut saved = self.saved.clone();
        saved.refresh_uri(&uri);
        self.refresh_open_identities(&saved, &mut documents);
        if self.rebuild_for_documents(saved, documents).is_err() {
            return Vec::new();
        }
        if self.is_schema_document_uri(&uri) {
            let mut refreshes = vec![DiagnosticRefresh::Clear {
                uri: uri.clone(),
                version: None,
                generation: self.generation,
            }];
            if let Some(refresh) = self.schema_refresh_for_uri(&uri) {
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
        let closed_partition = closed
            .identity()
            .saved_path
            .as_deref()
            .and_then(|path| self.saved.partition_for_path(path))
            .unwrap_or_else(|| "standalone".to_owned());
        let remaining_open = closed_key
            .as_ref()
            .and_then(|key| self.effective_open_document_for_partition_key(&closed_partition, key))
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
                    version: None,
                    generation: self.generation,
                }),
        ]
    }
}
