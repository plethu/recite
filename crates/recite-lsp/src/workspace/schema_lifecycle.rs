use super::{DiagnosticRefresh, DocumentDiagnostics, LspWorkspace, SchemaRefreshOutcome};

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

    pub(super) fn schema_authority_for_uri(&self, uri: &lsp_types::Uri) -> Option<lsp_types::Uri> {
        self.partitions
            .values()
            .filter(|partition| partition.schema.matches_uri(uri))
            .filter_map(|partition| partition.schema.protocol_uri())
            .min_by_key(|uri| uri.as_str().to_owned())
    }

    pub(super) fn schema_transition(
        &self,
        previous: Option<lsp_types::Uri>,
        outcome: SchemaRefreshOutcome,
    ) -> Vec<DiagnosticRefresh> {
        let SchemaRefreshOutcome::Refreshes(mut refreshes) = outcome else {
            return Vec::new();
        };
        let Some(previous) = previous else {
            return refreshes;
        };
        let next = refreshes.first().map(|refresh| match refresh {
            DiagnosticRefresh::Publish(published) => published.uri.clone(),
            DiagnosticRefresh::Clear { uri, .. } => uri.clone(),
        });
        if next.as_ref() != Some(&previous) {
            refreshes.insert(
                0,
                DiagnosticRefresh::Clear {
                    uri: previous,
                    version: None,
                    generation: self.generation,
                },
            );
        }
        refreshes
    }

    pub(crate) fn schema_refresh_for_uri(&self, uri: &lsp_types::Uri) -> SchemaRefreshOutcome {
        let owner = self.schema_owner_for_uri(uri);
        // A second alias opening must not republish the first owner's
        // diagnostics with the second document's version.  The deterministic
        // owner is refreshed when it opens or changes; a non-owner open has no
        // diagnostic transition to publish.
        if self.documents.document(uri).is_some()
            && owner.as_ref().is_some_and(|owner| owner != uri)
        {
            return SchemaRefreshOutcome::Silent;
        }
        let mut refreshes = self
            .partitions
            .values()
            .filter(|partition| partition.schema.matches_uri(uri))
            .filter_map(|partition| partition.schema.refresh_or_clear(self.generation));
        let first = refreshes.next();
        let Some(first) = first else {
            let Some(document) = self.documents.document(uri) else {
                return SchemaRefreshOutcome::NotSchema;
            };
            if self.is_retired_schema_alias(uri) {
                return SchemaRefreshOutcome::Refreshes(vec![DiagnosticRefresh::publish_open(
                    document,
                    Vec::new(),
                    self.generation,
                )]);
            }
            return SchemaRefreshOutcome::NotSchema;
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
            // When the previous owner has just closed, the refresh is for the
            // newly selected owner. Keep its URI, text, and version together;
            // never borrow payload fields from the closed/requested alias.
            if self.documents.document(uri).is_none()
                && let Some(owner) = owner
            {
                merged.uri = owner;
            }
            SchemaRefreshOutcome::Refreshes(vec![DiagnosticRefresh::Publish(merged)])
        } else {
            SchemaRefreshOutcome::Refreshes(vec![DiagnosticRefresh::Clear {
                uri: merged.uri,
                version: None,
                generation: self.generation,
            }])
        }
    }

    pub(super) fn schema_owner_for_uri(&self, uri: &lsp_types::Uri) -> Option<lsp_types::Uri> {
        let targets = self
            .partitions
            .values()
            .filter(|partition| partition.schema.matches_uri(uri))
            .filter_map(|partition| partition.schema.target_identity())
            .collect::<std::collections::BTreeSet<_>>();
        self.documents
            .documents()
            .filter(|document| {
                let Some(path) = document.identity().saved_path.as_deref() else {
                    return false;
                };
                targets.contains(&crate::paths::stable_path_identity(path))
            })
            .map(|document| document.identity().uri.clone())
            .next()
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
        let candidates = self
            .retired_schema_uris
            .iter()
            .chain(
                self.partitions
                    .values()
                    .flat_map(|partition| partition.retired_schema_uris.iter()),
            )
            .filter_map(|retired_uri| retired_uri.parse::<lsp_types::Uri>().ok())
            .filter_map(schema_target_id)
            .collect::<Vec<_>>();
        candidates.into_iter().any(|candidate| candidate == target)
    }
}

fn schema_target_id(uri: lsp_types::Uri) -> Option<String> {
    let path = crate::paths::uri_to_file_path(&uri)?;
    let path = std::fs::canonicalize(&path).unwrap_or(path);
    Some(crate::paths::stable_path_identity(&path))
}
