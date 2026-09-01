mod config;
mod document_keys;
mod kernel;
#[path = "workspace/kernel_rebuild.rs"]
mod kernel_rebuild;
#[path = "workspace/kernel_standalone.rs"]
mod kernel_standalone;
mod lsp_features;
mod partition_rollback;
mod project_index;
#[path = "project_refresh.rs"]
mod project_refresh;
mod schema_index;
mod schema_lifecycle;
mod snapshot;
mod transaction;
mod ui;

#[cfg(test)]
mod tests;

use std::collections::{BTreeMap, BTreeSet};

use lsp_types::Uri;
use recite_core::Diagnostic;
use recite_ui::UiCatalog;

pub(crate) use config::WorkspaceConfig;
pub(crate) use document_keys::{
    document_key_for_identity, document_key_for_open, document_key_for_saved,
};
pub(crate) use kernel::KernelPartition;
use project_index::{SavedDocument, SavedProjectIndex};
use schema_index::SchemaIndex;
pub(crate) use snapshot::LiveProjectSnapshot;

use crate::documents::{OpenDocument, OpenDocumentStore};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SnapshotGeneration(u64);

pub(crate) struct LspWorkspace {
    saved: SavedProjectIndex,
    documents: OpenDocumentStore,
    partitions: BTreeMap<String, KernelPartition>,
    snapshot: LiveProjectSnapshot,
    schema_override_path: Option<std::path::PathBuf>,
    schema_paths: BTreeMap<String, Option<std::path::PathBuf>>,
    retired_schema_uris: BTreeSet<String>,
    retired_schema_targets: BTreeMap<String, String>,
    generation: SnapshotGeneration,
    next_partition_build_id: u64,
    pub(crate) ui_catalog: UiCatalog,
}

impl LspWorkspace {
    pub(crate) fn schema_diagnostics_all(&self) -> Vec<DiagnosticRefresh> {
        let mut refreshes: Vec<(String, DocumentDiagnostics)> = Vec::new();
        for partition in self.partitions.values() {
            let Some(refresh) = partition.schema.diagnostics_refresh(self.generation) else {
                continue;
            };
            let DiagnosticRefresh::Publish(published) = refresh else {
                continue;
            };
            let key = partition
                .schema
                .target_identity()
                .unwrap_or_else(|| published.uri.as_str().to_owned());
            if let Some((_, existing)) = refreshes.iter_mut().find(|(existing, _)| *existing == key)
            {
                for diagnostic in published.diagnostics {
                    if !existing.diagnostics.contains(&diagnostic) {
                        existing.diagnostics.push(diagnostic);
                    }
                }
            } else {
                refreshes.push((key, published));
            }
        }
        refreshes
            .into_iter()
            .map(|(_, diagnostics)| DiagnosticRefresh::Publish(diagnostics))
            .collect()
    }

    pub(crate) fn save_schema(&mut self, uri: &Uri) -> Option<DiagnosticRefresh> {
        if self.schema_partition_ids(uri).is_empty() {
            return None;
        }
        let mut schemas = self.partition_schemas();
        for schema in schemas.values_mut() {
            if schema.matches_uri(uri) {
                *schema = schema.base();
            }
        }
        self.rebuild_for_documents_with_schemas(
            self.saved.clone(),
            self.documents.clone(),
            schemas,
        )
        .ok()?;
        let SchemaRefreshOutcome::Refreshes(mut refreshes) = self.schema_refresh_for_uri(uri)
        else {
            return None;
        };
        refreshes.pop()
    }

    pub(crate) fn is_current_generation(&self, generation: SnapshotGeneration) -> bool {
        self.generation == generation && self.snapshot.generation() == generation
    }

    #[cfg(any(test, feature = "bench-support"))]
    pub(crate) fn generation(&self) -> SnapshotGeneration {
        self.generation
    }

    #[cfg(any(test, feature = "bench-support"))]
    pub(crate) fn snapshot(&self) -> &LiveProjectSnapshot {
        &self.snapshot
    }

    #[cfg(feature = "bench-support")]
    pub(crate) fn compiler_documents(&self) -> Vec<&recite_compiler::DocumentSnapshot> {
        self.partitions
            .values()
            .flat_map(|partition| partition.kernel.snapshot().documents())
            .collect()
    }

    #[cfg(feature = "bench-support")]
    pub(crate) fn compiler_document_for_summary(
        &self,
        summary: &crate::summary::FileSummary,
    ) -> Option<&recite_compiler::DocumentSnapshot> {
        let partition = self.partition_id_for_uri(summary.uri())?;
        let key = document_key_for_identity(&summary.identity)?;
        self.partition(&partition)?.kernel.snapshot().document(&key)
    }

    #[allow(dead_code)]
    pub(crate) fn schema(&self) -> &SchemaIndex {
        let Some(partition) = self
            .partitions
            .values()
            .find(|partition| partition.schema.summary().is_some())
            .or_else(|| self.partitions.values().next())
        else {
            unreachable!("workspace always has a partition")
        };
        &partition.schema
    }

    pub(in crate::workspace) fn rebuild_for_documents(
        &mut self,
        saved: SavedProjectIndex,
        documents: OpenDocumentStore,
    ) -> Result<(), recite_compiler::AuthoringError> {
        let retired = self
            .partitions
            .iter()
            .map(|(id, partition)| (id.clone(), partition.retired_schema_uris.clone()))
            .collect();
        self.rebuild_for_documents_with_schemas_and_retired(
            saved,
            documents,
            self.partition_schemas(),
            retired,
        )
    }

    pub(crate) fn is_schema_document_uri(&self, uri: &Uri) -> bool {
        self.partitions.values().any(|partition| {
            partition.schema.matches_uri(uri)
                || partition.retired_schema_uris.contains(uri.as_str())
        }) || self.retired_schema_uris.contains(uri.as_str())
            || self.retired_schema_targets.contains_key(uri.as_str())
            || self.is_retired_schema_alias(uri)
    }
}

#[derive(Clone, Debug)]
pub(crate) enum WorkspaceChangeResult {
    Accepted(DiagnosticRefresh),
    AcceptedRefreshes(Vec<DiagnosticRefresh>),
    Stale,
    Malformed,
    Unopened,
    Rejected,
}

#[derive(Clone, Debug)]
pub(crate) enum SchemaRefreshOutcome {
    NotSchema,
    Silent,
    Refreshes(Vec<DiagnosticRefresh>),
}

#[derive(Clone, Debug)]
pub(crate) enum DiagnosticRefresh {
    Publish(DocumentDiagnostics),
    Clear {
        uri: Uri,
        version: Option<i32>,
        generation: SnapshotGeneration,
    },
}

impl DiagnosticRefresh {
    fn publish_open(
        document: &OpenDocument,
        diagnostics: Vec<Diagnostic>,
        generation: SnapshotGeneration,
    ) -> Self {
        Self::Publish(DocumentDiagnostics {
            uri: document.identity().uri.clone(),
            text: document.text().to_owned(),
            version: Some(document.version()),
            diagnostics,
            generation,
        })
    }

    fn publish_saved(
        document: &SavedDocument,
        diagnostics: Vec<Diagnostic>,
        generation: SnapshotGeneration,
    ) -> Self {
        Self::Publish(DocumentDiagnostics {
            uri: document.identity.uri.clone(),
            text: document.text.clone(),
            version: None,
            diagnostics,
            generation,
        })
    }

    pub(crate) fn generation(&self) -> SnapshotGeneration {
        match self {
            Self::Publish(diagnostics) => diagnostics.generation,
            Self::Clear { generation, .. } => *generation,
        }
    }
}

#[derive(Clone, Debug)]
pub(crate) struct DocumentDiagnostics {
    pub(crate) uri: Uri,
    pub(crate) text: String,
    pub(crate) version: Option<i32>,
    pub(crate) diagnostics: Vec<Diagnostic>,
    pub(crate) generation: SnapshotGeneration,
}
