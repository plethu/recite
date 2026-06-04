mod config;
mod project_index;
mod schema_index;

use std::fs;

use lsp_types::{CompletionResponse, Hover, Position, TextDocumentContentChangeEvent, Uri};
use recite_core::{Diagnostic, ProjectSchema};
use recite_parser::parse;

pub(crate) use config::WorkspaceConfig;
use project_index::{LiveProjectSnapshot, SavedDocument, SavedProjectIndex};
use schema_index::SchemaIndex;

use crate::documents::{DocumentChangeResult, OpenDocument, OpenDocumentStore};
use crate::features;
use crate::paths::{project_relative_path, uri_to_file_path};
use crate::summary::OpenFileIdentity;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct SnapshotGeneration(u64);

pub(crate) struct LspWorkspace {
    saved: SavedProjectIndex,
    documents: OpenDocumentStore,
    snapshot: LiveProjectSnapshot,
    schema: SchemaIndex,
    generation: SnapshotGeneration,
}

impl LspWorkspace {
    pub(crate) fn new(config: WorkspaceConfig) -> Self {
        let saved = SavedProjectIndex::discover(&config.roots);
        let schema = SchemaIndex::load(config.schema_path);
        let documents = OpenDocumentStore::default();
        let generation = SnapshotGeneration(0);
        let snapshot = LiveProjectSnapshot::rebuild(generation, &saved, &documents);

        Self {
            saved,
            documents,
            snapshot,
            schema,
            generation,
        }
    }

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

    pub(crate) fn save(&mut self, uri: Uri) -> Option<DiagnosticRefresh> {
        let touched_saved = self.saved.refresh_uri(&uri);
        let open_identity = self.open_identity(uri.clone());
        let open_refresh = self.documents.refresh_identity(open_identity);

        if touched_saved
            || open_refresh
                .as_ref()
                .is_some_and(|refresh| refresh.identity_changed)
        {
            self.rebuild_next_generation();
        }

        if let Some(open_refresh) = open_refresh {
            return Some(DiagnosticRefresh::publish_open(
                &open_refresh.document,
                self.generation,
            ));
        }

        self.saved
            .document_by_uri(&uri)
            .map(|document| DiagnosticRefresh::publish_saved(document, self.generation))
            .or_else(|| {
                touched_saved.then_some(DiagnosticRefresh::Clear {
                    uri,
                    generation: self.generation,
                })
            })
    }

    pub(crate) fn close(&mut self, uri: Uri) -> Option<DiagnosticRefresh> {
        self.documents.close(&uri)?;
        self.saved.refresh_uri(&uri);
        self.rebuild_next_generation();

        Some(
            self.saved
                .document_by_uri(&uri)
                .map(|document| DiagnosticRefresh::publish_saved(document, self.generation))
                .unwrap_or(DiagnosticRefresh::Clear {
                    uri,
                    generation: self.generation,
                }),
        )
    }

    pub(crate) fn schema_diagnostics(&self) -> Option<DiagnosticRefresh> {
        self.schema.diagnostics_refresh(self.generation)
    }

    pub(crate) fn completion(&self, uri: &Uri, position: Position) -> Option<CompletionResponse> {
        let text = self.documents.document(uri)?.text();
        let schema = self.schema.schema()?;
        features::completion(text, position, schema)
    }

    pub(crate) fn hover(&self, uri: &Uri, position: Position) -> Option<Hover> {
        let text = self.documents.document(uri)?.text();
        features::hover(text, position)
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

impl DocumentDiagnostics {
    pub(crate) fn with_schema_diagnostics(mut self, schema: Option<&ProjectSchema>) -> Self {
        if self.diagnostics.is_empty()
            && let Some(schema) = schema
        {
            let lowered = parse(self.uri.as_str(), self.text.as_str()).lower_source_file();
            if lowered.diagnostics.is_empty() {
                self.diagnostics = recite_compiler::validate_source_files_with_schema(
                    &[lowered.source_file],
                    schema,
                )
                .diagnostics;
            }
        }

        self
    }
}
