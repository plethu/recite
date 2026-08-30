mod config;
mod kernel;
mod project_index;
#[path = "project_refresh.rs"]
mod project_refresh;
mod schema_index;
mod snapshot;
mod transaction;
mod ui;

use std::collections::BTreeMap;
use std::fs;
use std::path::{Component, Path, PathBuf};

use lsp_types::{
    CodeActionParams, CodeActionResponse, CompletionResponse, GotoDefinitionResponse, Hover,
    Location, Position, PrepareRenameResponse, Uri, WorkspaceEdit,
};
use recite_compiler::{AuthoringKernel, DocumentLayer};
use recite_core::{Diagnostic, DocumentKey};
use recite_ui::UiCatalog;

pub(crate) use config::WorkspaceConfig;
pub(crate) use kernel::{document_key_for_identity, document_key_for_open, document_key_for_saved};
use project_index::{SavedDocument, SavedProjectIndex};
use schema_index::SchemaIndex;
pub(crate) use snapshot::LiveProjectSnapshot;

use crate::documents::{OpenDocument, OpenDocumentStore};
use crate::features;
use crate::position::lsp_position_to_source;
use crate::summary::{FileSummary, OpenFileIdentity};

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
        let mut schema = self.schema.clone();
        if !schema.refresh_uri(uri) {
            return None;
        }
        self.rebuild_state_with_schema(self.saved.clone(), self.documents.clone(), schema)
            .ok()?;
        self.schema.refresh_or_clear(self.generation)
    }

    pub(crate) fn completion(&self, uri: &Uri, position: Position) -> Option<CompletionResponse> {
        let document = self.documents.document(uri)?;
        let key = document_key_for_open(document);
        features::completion(
            document.text(),
            position,
            key.as_ref(),
            self.kernel.snapshot(),
            self.schema.schema(),
            self.schema.matches_uri(uri),
            &self.ui_catalog,
        )
    }

    pub(crate) fn hover(&self, uri: &Uri, position: Position) -> Option<Hover> {
        let document = self.documents.document(uri)?;
        let key = document_key_for_open(document)?;
        features::hover(
            document.text(),
            position,
            &key,
            self.kernel.snapshot(),
            self.schema.schema(),
            &self.ui_catalog,
        )
    }

    pub(crate) fn definition(
        &self,
        uri: &Uri,
        position: Position,
    ) -> Option<GotoDefinitionResponse> {
        let (key, text) = self.source_document(uri)?;
        let position = lsp_position_to_source(text, position)?;
        let documents = self.navigation_documents();
        features::definition(&key, position, self.kernel.snapshot(), &documents)
    }

    pub(crate) fn references(
        &self,
        uri: &Uri,
        position: Position,
        include_declaration: bool,
    ) -> Option<Vec<Location>> {
        let (key, text) = self.source_document(uri)?;
        let position = lsp_position_to_source(text, position)?;
        let documents = self.navigation_documents();
        features::references(
            &key,
            position,
            include_declaration,
            self.kernel.snapshot(),
            &documents,
        )
    }

    pub(crate) fn prepare_rename(
        &self,
        uri: &Uri,
        position: Position,
    ) -> Option<PrepareRenameResponse> {
        let (key, text) = self.source_document(uri)?;
        let position = lsp_position_to_source(text, position)?;
        features::prepare_rename(&key, position, self.kernel.snapshot())
    }

    pub(crate) fn rename(
        &self,
        uri: &Uri,
        position: Position,
        new_name: &str,
    ) -> Option<WorkspaceEdit> {
        let (key, text) = self.source_document(uri)?;
        let position = lsp_position_to_source(text, position)?;
        let documents = self.navigation_documents();
        features::rename(&key, position, new_name, self.kernel.snapshot(), &documents)
    }

    pub(crate) fn code_action(&self, params: &CodeActionParams) -> Option<CodeActionResponse> {
        let documents = self.code_action_documents();
        let schema_is_open = self
            .schema
            .uri()
            .is_some_and(|uri| self.documents.document(uri).is_some())
            || self.schema.path().is_some_and(|path| {
                self.documents.documents().any(|document| {
                    document
                        .identity()
                        .saved_path
                        .as_ref()
                        .is_some_and(|open| open == path)
                })
            });
        let schema_document = if schema_is_open {
            None
        } else {
            self.schema.code_action_document()
        };
        features::code_action(params, &documents, schema_document, &self.ui_catalog)
    }

    pub(crate) fn open_document_diagnostics_except(
        &self,
        exclude: Option<&Uri>,
    ) -> Vec<DiagnosticRefresh> {
        self.documents
            .documents()
            .filter(|document| match exclude {
                Some(uri) => document.identity().uri != *uri,
                None => true,
            })
            .map(|document| self.publish_open_document(document))
            .collect()
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

    #[allow(dead_code)]
    pub(crate) fn schema(&self) -> &SchemaIndex {
        &self.schema
    }

    fn navigation_documents(&self) -> Vec<features::NavigationDocument<'_>> {
        self.kernel
            .snapshot()
            .documents()
            .iter()
            .filter_map(|document| {
                let uri = self.uri_for_document_key(document.key())?;
                Some(features::NavigationDocument {
                    uri,
                    key: document.key(),
                    text: document.source_text(),
                })
            })
            .collect()
    }

    fn source_document(&self, uri: &Uri) -> Option<(DocumentKey, &str)> {
        if let Some(document) = self.documents.document(uri) {
            return Some((document_key_for_open(document)?, document.text()));
        }
        let document = self.saved.document_by_uri(uri)?;
        Some((document_key_for_saved(document)?, document.text.as_str()))
    }

    fn uri_for_document_key(&self, key: &DocumentKey) -> Option<&Uri> {
        let document = self.kernel.snapshot().document(key)?;
        match document.layer() {
            DocumentLayer::Open => self.kernel_open_owners.get(key),
            DocumentLayer::Saved => self
                .saved
                .documents
                .values()
                .find(|document| document_key_for_saved(document).as_ref() == Some(key))
                .map(|document| &document.identity.uri),
            _ => None,
        }
    }

    fn code_action_documents(&self) -> Vec<features::CodeActionDocument<'_>> {
        self.snapshot
            .summaries()
            .iter()
            .filter_map(|summary| {
                let text = self.text_for_summary(summary)?;
                Some(features::CodeActionDocument {
                    uri: summary.uri(),
                    text,
                    summary,
                })
            })
            .collect()
    }

    fn text_for_summary(&self, summary: &FileSummary) -> Option<&str> {
        self.documents
            .document(summary.uri())
            .map(OpenDocument::text)
            .or_else(|| {
                self.saved
                    .document_by_uri(summary.uri())
                    .map(|document| document.text.as_str())
            })
    }
}

fn canonical_or_normalized_path(path: &Path) -> (PathBuf, bool) {
    if let Ok(canonical_path) = fs::canonicalize(path) {
        return (canonical_path, true);
    }

    let normalized = lexically_normalized_path(path);
    let mut missing_components = Vec::new();
    let mut cursor = normalized.as_path();
    loop {
        if let Ok(canonical_parent) = fs::canonicalize(cursor) {
            let mut path = canonical_parent;
            for component in missing_components.iter().rev() {
                path.push(component);
            }
            return (path, false);
        }

        let Some(component) = cursor.file_name() else {
            return (normalized, false);
        };
        missing_components.push(component.to_owned());
        let Some(parent) = cursor.parent() else {
            return (normalized, false);
        };
        cursor = parent;
    }
}

fn lexically_normalized_path(path: &Path) -> PathBuf {
    let mut normalized = PathBuf::new();
    for component in path.components() {
        match component {
            Component::Prefix(prefix) => normalized.push(prefix.as_os_str()),
            Component::RootDir => normalized.push(Path::new(std::path::MAIN_SEPARATOR_STR)),
            Component::CurDir => {}
            Component::ParentDir => {
                normalized.pop();
            }
            Component::Normal(component) => normalized.push(component),
        }
    }
    normalized
}

fn uri_keyed_open_identity(uri: Uri) -> OpenFileIdentity {
    OpenFileIdentity {
        uri,
        saved_path: None,
        project_relative_path: None,
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
