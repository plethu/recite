use super::{DiagnosticRefresh, DocumentDiagnostics, LspWorkspace};

impl LspWorkspace {
    pub(crate) fn schema_partition_id(&self, uri: &lsp_types::Uri) -> Option<String> {
        self.schema_partition_ids(uri).into_iter().next()
    }

    pub(crate) fn schema_partition_ids(&self, uri: &lsp_types::Uri) -> Vec<String> {
        self.partitions
            .iter()
            .filter(|(_, partition)| partition.schema.matches_uri(uri))
            .map(|(id, _)| id.clone())
            .collect()
    }

    pub(crate) fn schema_refresh_for_uri(&self, uri: &lsp_types::Uri) -> Option<DiagnosticRefresh> {
        let mut refreshes = self
            .partitions
            .values()
            .filter(|partition| partition.schema.matches_uri(uri))
            .filter_map(|partition| partition.schema.refresh_or_clear(self.generation));
        let first = refreshes.next();
        let Some(first) = first else {
            let document = self.documents.document(uri)?;
            return Some(DiagnosticRefresh::publish_open(
                document,
                Vec::new(),
                self.generation,
            ));
        };
        let mut has_publish = matches!(&first, DiagnosticRefresh::Publish(_));
        let mut merged = match first {
            DiagnosticRefresh::Publish(published) => published,
            DiagnosticRefresh::Clear { uri, .. } => DocumentDiagnostics {
                uri,
                text: String::new(),
                version: None,
                diagnostics: Vec::new(),
                generation: self.generation,
            },
        };
        for refresh in refreshes {
            match refresh {
                DiagnosticRefresh::Publish(published) => {
                    has_publish = true;
                    if merged.text.is_empty() {
                        merged.text = published.text;
                    }
                    merged.version = merged.version.or(published.version);
                    for diagnostic in published.diagnostics {
                        if !merged.diagnostics.contains(&diagnostic) {
                            merged.diagnostics.push(diagnostic);
                        }
                    }
                }
                DiagnosticRefresh::Clear { .. } => {}
            }
        }
        if has_publish {
            if let Some(document) = self.documents.document(uri) {
                merged.text = document.text().to_owned();
                merged.version = Some(document.version());
            }
            Some(DiagnosticRefresh::Publish(merged))
        } else {
            Some(DiagnosticRefresh::Clear {
                uri: merged.uri,
                version: None,
                generation: self.generation,
            })
        }
    }

    pub(super) fn is_retired_schema_alias(&self, uri: &lsp_types::Uri) -> bool {
        let Some(target) = self
            .documents
            .document(uri)
            .and_then(|document| document.identity().saved_path.as_deref())
            .map(crate::paths::stable_path_identity)
        else {
            return false;
        };
        self.documents.documents().any(|document| {
            let retired = self
                .retired_schema_uris
                .contains(document.identity().uri.as_str())
                || self.partitions.values().any(|partition| {
                    partition
                        .retired_schema_uris
                        .contains(document.identity().uri.as_str())
                });
            retired
                && document
                    .identity()
                    .saved_path
                    .as_deref()
                    .map(crate::paths::stable_path_identity)
                    .is_some_and(|candidate| candidate == target)
        })
    }
}
