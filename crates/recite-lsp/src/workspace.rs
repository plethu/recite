mod config;
mod kernel;
mod lsp_features;
mod project_index;
#[path = "project_refresh.rs"]
mod project_refresh;
mod schema_index;
mod snapshot;
mod transaction;
mod ui;

use std::collections::BTreeMap;

use lsp_types::Uri;
use recite_compiler::AuthoringKernel;
use recite_core::{Diagnostic, DocumentKey};
use recite_ui::UiCatalog;

pub(crate) use config::WorkspaceConfig;
pub(crate) use kernel::{document_key_for_identity, document_key_for_open, document_key_for_saved};
use project_index::{SavedDocument, SavedProjectIndex};
use schema_index::SchemaIndex;
pub(crate) use snapshot::LiveProjectSnapshot;

use crate::documents::{OpenDocument, OpenDocumentStore};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SnapshotGeneration(u64);

pub(crate) struct LspWorkspace {
    saved: SavedProjectIndex,
    documents: OpenDocumentStore,
    kernel: AuthoringKernel,
    kernel_open_owners: BTreeMap<DocumentKey, Uri>,
    snapshot: LiveProjectSnapshot,
    schema: SchemaIndex,
    generation: SnapshotGeneration,
    pub(crate) ui_catalog: UiCatalog,
}

impl LspWorkspace {
    pub(crate) fn schema_diagnostics(&self) -> Option<DiagnosticRefresh> {
        self.schema.diagnostics_refresh(self.generation)
    }

    pub(crate) fn save_schema(&mut self, uri: &Uri) -> Option<DiagnosticRefresh> {
        if !self.schema.matches_uri(uri) {
            return None;
        }
        // Reload disk as the base, then let the live document store reapply
        // the authoritative unsaved owner before rebuilding the kernel.
        self.schema = self.schema.base();
        self.rebuild_for_documents(self.saved.clone(), self.documents.clone())
            .ok()?;
        self.schema.refresh_or_clear(self.generation)
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
    pub(crate) fn compiler_snapshot(&self) -> &recite_compiler::AuthoringSnapshot {
        self.kernel.snapshot()
    }

    #[allow(dead_code)]
    pub(crate) fn schema(&self) -> &SchemaIndex {
        &self.schema
    }

    pub(in crate::workspace) fn rebuild_for_documents(
        &mut self,
        saved: SavedProjectIndex,
        documents: OpenDocumentStore,
    ) -> Result<(), recite_compiler::AuthoringError> {
        if let Some(schema) = self.schema.overlay_for_documents(&documents) {
            return self.rebuild_state_with_schema(saved, documents, schema);
        }
        if self.schema.has_open_match(&documents) {
            let Some(uri) = documents
                .documents()
                .find(|document| self.schema.matches_uri(&document.identity().uri))
                .map(|document| document.identity().uri.clone())
            else {
                return self.rebuild_for_documents(saved, documents);
            };
            return self.rebuild_state_with_schema(
                saved,
                documents,
                self.schema.unavailable_overlay(uri),
            );
        }
        let base = self.schema.base();
        self.rebuild_state_with_schema(saved, documents, base)
    }
}

#[derive(Clone, Debug)]
pub(crate) enum WorkspaceChangeResult {
    Accepted(DiagnosticRefresh),
    Stale,
    Malformed,
    Unopened,
    Rejected,
}

#[derive(Clone, Debug)]
pub(crate) enum DiagnosticRefresh {
    Publish(DocumentDiagnostics),
    Clear {
        uri: Uri,
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
