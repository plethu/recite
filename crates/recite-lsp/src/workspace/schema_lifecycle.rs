use super::{DiagnosticRefresh, DocumentDiagnostics, LspWorkspace};

impl LspWorkspace {
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
                merged.uri = uri.clone();
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
}
