use std::collections::{BTreeMap, BTreeSet};

use lsp_types::Uri;

use super::schema_index::SchemaIndex;
use super::{DiagnosticRefresh, LspWorkspace};

#[path = "project_refresh/project_refresh_support.rs"]
mod project_refresh_support;
use project_refresh_support::{clear_old_schema, manifest_refreshes, schema_paths_for_saved};

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
        } else if let Some(document) = self.saved.document_by_uri(&uri) {
            refreshes.push(self.publish_saved_document(document));
        } else if touched_saved {
            refreshes.push(DiagnosticRefresh::Clear {
                uri,
                version: None,
                generation: self.generation,
            });
        }
        refreshes
    }

    pub(crate) fn refresh_project_manifest(&mut self) -> Vec<DiagnosticRefresh> {
        let old_document_uris = self.saved.document_uris().cloned().collect::<Vec<_>>();
        let old_manifest_diagnostics = self.saved.manifest_diagnostics().clone();
        let mut old_retired: BTreeMap<String, BTreeSet<String>> = self
            .partitions
            .iter()
            .map(|(id, partition)| (id.clone(), partition.retired_schema_uris.clone()))
            .collect();
        let old_partitions = std::mem::take(&mut self.partitions);
        let old_documents = self.documents.clone();
        let mut saved = self.saved.clone();
        saved.refresh_manifests();
        let schema_paths = schema_paths_for_saved(&saved, self.schema_override_path.as_ref());
        let schemas = schema_paths
            .iter()
            .map(|(id, path)| (id.clone(), SchemaIndex::load(path.clone())))
            .collect::<BTreeMap<_, _>>();
        for (id, old) in old_partitions.iter() {
            let changed = schemas
                .get(id)
                .is_some_and(|new| old.schema.configured_path() != new.configured_path());
            if changed {
                old_retired.entry(id.clone()).or_default().extend(
                    old_documents
                        .documents()
                        .filter(|document| old.schema.matches_uri(&document.identity().uri))
                        .map(|document| document.identity().uri.as_str().to_owned()),
                );
            }
        }
        let mut documents = self.documents.clone();
        self.refresh_open_identities(&saved, &mut documents);
        if self
            .rebuild_for_documents_with_schemas_and_retired(saved, documents, schemas, old_retired)
            .is_err()
        {
            self.partitions = old_partitions;
            return Vec::new();
        }
        self.schema_paths = schema_paths;

        let mut refreshes = manifest_refreshes(self, &old_manifest_diagnostics);
        for (id, old) in old_partitions {
            let Some(new) = self.partitions.get_mut(&id) else {
                refreshes.extend(clear_old_schema(self, &old.schema, &old_documents));
                continue;
            };
            if old.schema.configured_path() != new.schema.configured_path() {
                let old_open = old_documents
                    .documents()
                    .filter(|document| old.schema.matches_uri(&document.identity().uri))
                    .collect::<Vec<_>>();
                new.retired_schema_uris.extend(
                    old_open
                        .iter()
                        .map(|document| document.identity().uri.as_str().to_owned()),
                );
                refreshes.extend(old_open.into_iter().map(|document| {
                    DiagnosticRefresh::publish_open(document, Vec::new(), self.generation)
                }));
                if old_documents
                    .documents()
                    .all(|document| !old.schema.matches_uri(&document.identity().uri))
                    && let Some(refresh) = old.schema.clear_refresh(self.generation)
                {
                    refreshes.push(refresh);
                }
            }
            if (new.schema.needs_refresh() || old.schema.needs_refresh())
                && let Some(refresh) = new.schema.refresh_or_clear(self.generation)
            {
                refreshes.push(refresh);
            }
        }
        for uri in old_document_uris {
            if self.saved.document_by_uri(&uri).is_none() && self.documents.document(&uri).is_none()
            {
                refreshes.push(DiagnosticRefresh::Clear {
                    uri,
                    version: None,
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
        if self.schema_partition_id(uri).is_some()
            && let Some(refresh) = self.save_schema(uri)
        {
            return vec![refresh];
        }
        let mut saved = self.saved.clone();
        let mut documents = self.documents.clone();
        let touched_saved = saved.refresh_uri(uri);
        let open_identity_changed = self.refresh_open_identities(&saved, &mut documents);
        if !touched_saved && !open_identity_changed {
            return Vec::new();
        }
        let watched_keys = self.watched_document_keys(uri, &saved, &documents);
        if self.rebuild_for_documents(saved, documents).is_err() {
            return Vec::new();
        }
        if let Some(document) = watched_keys.iter().find_map(|(partition, key)| {
            self.effective_open_document_for_partition_key(partition, key)
        }) {
            return vec![self.publish_open_document(document)];
        }
        self.saved
            .document_by_uri(uri)
            .map(|document| vec![self.publish_saved_document(document)])
            .unwrap_or_else(|| {
                vec![DiagnosticRefresh::Clear {
                    uri: uri.clone(),
                    version: None,
                    generation: self.generation,
                }]
            })
    }

    fn watched_document_keys(
        &self,
        uri: &Uri,
        saved: &super::project_index::SavedProjectIndex,
        documents: &crate::documents::OpenDocumentStore,
    ) -> BTreeSet<(String, recite_core::DocumentKey)> {
        documents
            .document(uri)
            .and_then(|document| {
                super::document_key_for_open(document).map(|key| {
                    (
                        document
                            .identity()
                            .saved_path
                            .as_deref()
                            .and_then(|path| saved.partition_for_path(path))
                            .unwrap_or_else(|| "standalone".to_owned()),
                        key,
                    )
                })
            })
            .into_iter()
            .chain(saved.document_by_uri(uri).and_then(|document| {
                super::document_key_for_saved(document).map(|key| {
                    (
                        saved
                            .partition_for_path(&document.identity.canonical_path)
                            .unwrap_or_else(|| "standalone".to_owned()),
                        key,
                    )
                })
            }))
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
