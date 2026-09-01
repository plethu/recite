use std::collections::{BTreeMap, BTreeSet};

use lsp_types::Uri;

use super::config::schema_paths_for_saved;
use super::schema_index::SchemaIndex;
use super::{DiagnosticRefresh, LspWorkspace};

#[path = "project_refresh/close.rs"]
mod close;
#[path = "project_refresh/project_refresh_support.rs"]
mod project_refresh_support;
#[path = "project_refresh/retired_schema.rs"]
mod retired_schema;
#[path = "project_refresh/schema_authority.rs"]
mod schema_authority;
use project_refresh_support::{clear_old_schema, coalesce_refreshes, manifest_refreshes};
use retired_schema::update_retired_schema_state;
use schema_authority::carry_schema_authorities;

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
        let old_partition_ids = self.partitions.keys().cloned().collect::<BTreeSet<_>>();
        let old_schemas = self
            .partitions
            .iter()
            .map(|(id, partition)| (id.clone(), partition.schema.clone()))
            .collect::<BTreeMap<_, _>>();
        let old_retired_workspace = self.retired_schema_uris.clone();
        let old_documents = self.documents.clone();
        let mut saved = self.saved.clone();
        saved.refresh_manifests();
        let schema_paths = schema_paths_for_saved(&saved, self.schema_override_path.as_ref());
        let mut schemas = schema_paths
            .iter()
            .map(|(id, path)| (id.clone(), SchemaIndex::load(path.clone())))
            .collect::<BTreeMap<_, _>>();
        carry_schema_authorities(&old_schemas, &mut schemas);
        for (id, old) in &old_schemas {
            let changed = schemas
                .get(id)
                .is_some_and(|new| old.configured_path() != new.configured_path());
            if changed {
                old_retired.entry(id.clone()).or_default().extend(
                    old_documents
                        .documents()
                        .filter(|document| old.matches_uri(&document.identity().uri))
                        .map(|document| document.identity().uri.as_str().to_owned()),
                );
            }
        }
        let (old_retired, retired_workspace) = update_retired_schema_state(
            &self.partitions,
            &old_documents,
            &schemas,
            old_retired,
            old_retired_workspace.clone(),
        );
        self.retired_schema_uris = retired_workspace;
        let mut documents = self.documents.clone();
        self.refresh_open_identities(&saved, &mut documents);
        if self
            .rebuild_for_documents_with_schemas_and_retired(saved, documents, schemas, old_retired)
            .is_err()
        {
            self.retired_schema_uris = old_retired_workspace;
            return Vec::new();
        }
        self.schema_paths = schema_paths;

        let mut refreshes = manifest_refreshes(self, &old_manifest_diagnostics);
        for (id, old) in old_schemas {
            let Some(new) = self.partitions.get_mut(&id) else {
                refreshes.extend(clear_old_schema(self, &old, &old_documents));
                continue;
            };
            if old.configured_path() != new.schema.configured_path() {
                let old_open = old_documents
                    .documents()
                    .filter(|document| old.matches_uri(&document.identity().uri))
                    .collect::<Vec<_>>();
                new.retired_schema_uris.extend(
                    old_open
                        .iter()
                        .map(|document| document.identity().uri.as_str().to_owned()),
                );
                let protocol_uri = old.protocol_uri();
                refreshes.extend(old_open.into_iter().map(|document| {
                    let mut refresh =
                        DiagnosticRefresh::publish_open(document, Vec::new(), self.generation);
                    if let Some(protocol_uri) = &protocol_uri
                        && let DiagnosticRefresh::Publish(published) = &mut refresh
                    {
                        published.uri = protocol_uri.clone();
                    }
                    refresh
                }));
                if old_documents
                    .documents()
                    .all(|document| !old.matches_uri(&document.identity().uri))
                    && let Some(refresh) = old.clear_refresh(self.generation)
                {
                    refreshes.push(refresh);
                }
            }
            if (new.schema.needs_refresh() || old.needs_refresh())
                && let Some(refresh) = new.schema.refresh_or_clear(self.generation)
            {
                refreshes.push(refresh);
            }
        }
        for (id, partition) in &self.partitions {
            if !old_partition_ids.contains(id)
                && partition.schema.needs_refresh()
                && let Some(refresh) = partition.schema.refresh_or_clear(self.generation)
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
        coalesce_refreshes(refreshes)
    }

    pub(crate) fn refresh_watched_uri(&mut self, uri: &Uri) -> Vec<DiagnosticRefresh> {
        if crate::paths::uri_to_file_path(uri)
            .is_some_and(|path| self.saved.is_manifest_candidate(&path))
        {
            return self.refresh_project_manifest();
        }
        if !self.schema_partition_ids(uri).is_empty()
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
                            .and_then(|path| saved.partition_for_open_path(path))
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
}
