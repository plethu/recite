mod config;
mod project_index;
#[path = "project_refresh.rs"]
mod project_refresh;
mod schema_index;
#[path = "semantic.rs"]
mod semantic;
mod ui;

use std::fs;

use lsp_types::{
    CodeActionParams, CodeActionResponse, CompletionResponse, GotoDefinitionResponse, Hover,
    Location, Position, PrepareRenameResponse, TextDocumentContentChangeEvent, Uri, WorkspaceEdit,
};
use recite_core::Diagnostic;
use recite_ui::UiCatalog;

pub(crate) use config::WorkspaceConfig;
pub(crate) use project_index::LiveProjectSnapshot;
use project_index::{SavedDocument, SavedProjectIndex};
use schema_index::SchemaIndex;

use crate::documents::{DocumentChangeResult, OpenDocument, OpenDocumentStore};
use crate::features;
use crate::paths::{project_relative_path, uri_to_file_path};
use crate::summary::{FileSummary, OpenFileIdentity};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SnapshotGeneration(u64);

pub(crate) struct LspWorkspace {
    saved: SavedProjectIndex,
    documents: OpenDocumentStore,
    snapshot: LiveProjectSnapshot,
    schema: SchemaIndex,
    generation: SnapshotGeneration,
    pub(crate) ui_catalog: UiCatalog,
}

impl LspWorkspace {
    pub(crate) fn open(&mut self, uri: Uri, version: i32, text: String) -> DiagnosticRefresh {
        let identity = self.open_identity(uri);
        let document = self.documents.open(identity, version, text);
        self.rebuild_next_generation();
        DiagnosticRefresh::publish_open(&document, self.generation)
    }

    pub(crate) fn change(
        &mut self,
        uri: Uri,
        version: i32,
        changes: Vec<TextDocumentContentChangeEvent>,
    ) -> WorkspaceChangeResult {
        let identity = self.open_identity(uri);
        match self.documents.change(identity, version, changes) {
            DocumentChangeResult::Accepted(document) => {
                self.rebuild_next_generation();
                WorkspaceChangeResult::Accepted(DiagnosticRefresh::publish_open(
                    &document,
                    self.generation,
                ))
            }
            DocumentChangeResult::Stale => WorkspaceChangeResult::Stale,
            DocumentChangeResult::Malformed => WorkspaceChangeResult::Malformed,
            DocumentChangeResult::Unopened => WorkspaceChangeResult::Unopened,
        }
    }

    pub(crate) fn schema_diagnostics(&self) -> Option<DiagnosticRefresh> {
        self.schema.diagnostics_refresh(self.generation)
    }

    pub(crate) fn save_schema(&mut self, uri: &Uri) -> Option<DiagnosticRefresh> {
        if !self.schema.refresh_uri(uri) {
            return None;
        }
        self.rebuild_next_generation();
        self.schema.refresh_or_clear(self.generation)
    }

    pub(crate) fn completion(&self, uri: &Uri, position: Position) -> Option<CompletionResponse> {
        let text = self.documents.document(uri)?.text();
        let schema = self.schema.schema()?;
        features::completion(
            text,
            position,
            schema,
            self.schema.matches_uri(uri),
            &self.snapshot,
            &self.ui_catalog,
        )
    }

    pub(crate) fn hover(&self, uri: &Uri, position: Position) -> Option<Hover> {
        let text = self.documents.document(uri)?.text();
        features::hover(
            text,
            position,
            self.schema.schema(),
            &self.snapshot,
            &self.ui_catalog,
        )
    }

    pub(crate) fn definition(
        &self,
        uri: &Uri,
        position: Position,
    ) -> Option<GotoDefinitionResponse> {
        let documents = self.navigation_documents();
        features::definition(uri, position, &documents)
    }

    pub(crate) fn references(
        &self,
        uri: &Uri,
        position: Position,
        include_declaration: bool,
    ) -> Option<Vec<Location>> {
        let documents = self.navigation_documents();
        features::references(uri, position, include_declaration, &documents)
    }

    pub(crate) fn prepare_rename(
        &self,
        uri: &Uri,
        position: Position,
    ) -> Option<PrepareRenameResponse> {
        let documents = self.navigation_documents();
        features::prepare_rename(uri, position, &documents)
    }

    pub(crate) fn rename(
        &self,
        uri: &Uri,
        position: Position,
        new_name: &str,
    ) -> Option<WorkspaceEdit> {
        let documents = self.navigation_documents();
        features::rename(uri, position, new_name, &documents)
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
                        .summary()
                        .saved_path()
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
            .map(|document| DiagnosticRefresh::publish_open(document, self.generation))
            .collect()
    }

    pub(crate) fn is_current_generation(&self, generation: SnapshotGeneration) -> bool {
        self.generation == generation
    }

    #[allow(dead_code)]
    pub(crate) fn generation(&self) -> SnapshotGeneration {
        self.generation
    }

    #[allow(dead_code)]
    pub(crate) fn snapshot(&self) -> &LiveProjectSnapshot {
        &self.snapshot
    }

    #[allow(dead_code)]
    pub(crate) fn schema(&self) -> &SchemaIndex {
        &self.schema
    }

    fn rebuild_next_generation(&mut self) {
        self.generation = SnapshotGeneration(self.generation.0.saturating_add(1));
        self.snapshot = LiveProjectSnapshot::rebuild(self.generation, &self.saved, &self.documents);
    }

    fn navigation_documents(&self) -> Vec<features::NavigationDocument<'_>> {
        self.snapshot
            .summaries()
            .iter()
            .filter_map(|summary| {
                let text = self.text_for_summary(summary)?;
                Some(features::NavigationDocument {
                    uri: summary.uri(),
                    project_relative_path: summary.project_relative_path(),
                    text,
                    summary,
                })
            })
            .collect()
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

    fn validation_path_for_uri(&self, uri: &Uri) -> String {
        if let Some(document) = self.documents.document(uri) {
            return document
                .summary()
                .project_relative_path()
                .unwrap_or(uri.as_str())
                .to_owned();
        }
        if let Some(document) = self.saved.document_by_uri(uri) {
            return document
                .summary
                .project_relative_path()
                .unwrap_or(uri.as_str())
                .to_owned();
        }

        uri.as_str().to_owned()
    }

    fn open_identity(&self, uri: Uri) -> OpenFileIdentity {
        let Some(path) = uri_to_file_path(&uri) else {
            return uri_keyed_open_identity(uri);
        };
        let Some(canonical_path) = fs::canonicalize(path).ok() else {
            return uri_keyed_open_identity(uri);
        };
        let Some(root) = self.saved.root_for_path(&canonical_path) else {
            return uri_keyed_open_identity(uri);
        };

        OpenFileIdentity {
            uri,
            saved_path: Some(canonical_path.clone()),
            project_relative_path: project_relative_path(root, &canonical_path),
        }
    }
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
    fn publish_open(document: &OpenDocument, generation: SnapshotGeneration) -> Self {
        Self::Publish(DocumentDiagnostics {
            uri: document.identity().uri.clone(),
            text: document.text().to_owned(),
            version: Some(document.version()),
            diagnostics: document.diagnostics().to_vec(),
            generation,
        })
    }

    fn publish_saved(document: &SavedDocument, generation: SnapshotGeneration) -> Self {
        Self::Publish(DocumentDiagnostics {
            uri: document.summary.uri().clone(),
            text: document.text.clone(),
            version: None,
            diagnostics: document.summary.diagnostics.clone(),
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
