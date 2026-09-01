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
        let old_retired_workspace = self.retired_schema_uris.clone();
        self.retired_schema_uris.remove(uri.as_str());
        let mut retired = self
            .partitions
            .iter()
            .map(|(id, partition)| (id.clone(), partition.retired_schema_uris.clone()))
            .collect::<std::collections::BTreeMap<_, _>>();
        for uris in retired.values_mut() {
            uris.remove(uri.as_str());
        }
        if self
            .rebuild_for_documents_with_schemas_and_retired(
                saved,
                documents,
                self.partition_schemas(),
                retired,
            )
            .is_err()
        {
            self.retired_schema_uris = old_retired_workspace;
            return Vec::new();
        }
        if self.is_schema_document_uri(&uri) {
            return self
                .schema_refresh_for_uri(&uri)
                .map_or_else(Vec::new, |refresh| vec![refresh]);
        }
        let closed_partition = closed
            .identity()
            .saved_path
            .as_deref()
            .and_then(|path| self.saved.partition_for_open_path(path))
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
