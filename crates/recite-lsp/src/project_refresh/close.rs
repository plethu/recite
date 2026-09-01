use super::super::{DiagnosticRefresh, LspWorkspace};

impl LspWorkspace {
    pub(crate) fn close(&mut self, uri: lsp_types::Uri) -> Vec<DiagnosticRefresh> {
        let old_schema_owner = self.schema_owner_for_uri(&uri);
        let mut documents = self.documents.clone();
        let Some(closed) = documents.close(&uri) else {
            return Vec::new();
        };
        let closed_key = super::super::document_key_for_open(&closed);
        let mut saved = self.saved.clone();
        saved.refresh_uri(&uri);
        self.refresh_open_identities(&saved, &mut documents);
        let was_retired = self.retired_schema_uris.contains(uri.as_str())
            || self
                .partitions
                .values()
                .any(|partition| partition.retired_schema_uris.contains(uri.as_str()));
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
            // Only the target's active owner had published schema diagnostics.
            // Closing another open alias must not republish the owner under a
            // closed URI (or clear an URI that was never published).
            if old_schema_owner.as_ref() != Some(&uri) {
                return Vec::new();
            }
            let remaining_schema_owner = self.schema_owner_for_uri(&uri);
            return self
                .schema_refresh_for_uri(&uri)
                .map_or_else(Vec::new, |refresh| {
                    if remaining_schema_owner.is_some() {
                        vec![
                            DiagnosticRefresh::Clear {
                                uri: uri.clone(),
                                version: None,
                                generation: self.generation,
                            },
                            refresh,
                        ]
                    } else {
                        vec![refresh]
                    }
                });
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
        let closed_refresh = was_retired.then_some(DiagnosticRefresh::Clear {
            uri: uri.clone(),
            version: None,
            generation: self.generation,
        });
        if let Some(document) = remaining_open {
            return closed_refresh
                .into_iter()
                .chain(std::iter::once(self.publish_open_document(document)))
                .collect();
        }
        if let Some(closed_refresh) = closed_refresh {
            return vec![closed_refresh];
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
