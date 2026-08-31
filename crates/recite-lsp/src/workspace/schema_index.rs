use std::fs;
use std::path::PathBuf;

use lsp_types::Uri;
use recite_core::{Diagnostic, ProjectSchema, SchemaSource, load_schema_manifest_str};

use crate::documents::OpenDocumentStore;
use crate::paths::file_path_to_uri;
use crate::summary::SchemaSummary;

mod io;
use io::{schema_io_diagnostic, schema_kind, schema_unavailable_diagnostic};
mod lifecycle;

#[derive(Clone)]
pub(crate) struct SchemaIndex {
    uri: Option<Uri>,
    configured_uri: Option<Uri>,
    configured_path: Option<PathBuf>,
    path: Option<PathBuf>,
    kind: SchemaKind,
    active_version: Option<i32>,
    summary: Option<SchemaSummary>,
    schema: Option<ProjectSchema>,
    source: Option<SchemaSource>,
    diagnostics: Vec<Diagnostic>,
    text: Option<String>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SchemaKind {
    Toml,
    Json,
    Unknown,
}

impl SchemaIndex {
    pub(super) fn load(path: Option<PathBuf>) -> Self {
        let Some(path) = path else {
            return Self {
                uri: None,
                configured_uri: None,
                configured_path: None,
                path: None,
                kind: SchemaKind::Unknown,
                active_version: None,
                summary: None,
                schema: None,
                source: None,
                diagnostics: Vec::new(),
                text: None,
            };
        };
        let kind = schema_kind(&path);
        let configured_uri = file_path_to_uri(&path);
        let path = fs::canonicalize(&path).unwrap_or(path);
        let uri = configured_uri.clone().or_else(|| file_path_to_uri(&path));
        let display_path = path.display().to_string();
        let text = match fs::read_to_string(&path) {
            Ok(source) => source,
            Err(error) => {
                return Self {
                    uri,
                    configured_uri: configured_uri.or_else(|| file_path_to_uri(&path)),
                    configured_path: Some(path.clone()),
                    path: Some(path),
                    kind,
                    active_version: None,
                    summary: None,
                    schema: None,
                    source: None,
                    diagnostics: schema_io_diagnostic(display_path, &error),
                    text: None,
                };
            }
        };

        let mut index = Self::from_text(path.clone(), kind, &text);
        index.uri = uri;
        index.configured_uri = configured_uri;
        index.configured_path = Some(path);
        index
    }

    fn from_text(path: PathBuf, kind: SchemaKind, text: &str) -> Self {
        let display_path = path.display().to_string();
        let (schema, source, summary, diagnostics) = match kind {
            SchemaKind::Toml => {
                let report = SchemaSource::load_str(display_path, text);
                let summary = report.source.as_ref().map(SchemaSummary::from_source);
                let schema = report.source.as_ref().map(|source| source.schema().clone());
                (schema, report.source, summary, report.diagnostics)
            }
            SchemaKind::Json => {
                let report = load_schema_manifest_str(display_path, text);
                let summary = report.schema.as_ref().map(SchemaSummary::from_schema);
                (report.schema, None, summary, report.diagnostics)
            }
            SchemaKind::Unknown => (
                None,
                None,
                None,
                schema_unavailable_diagnostic(display_path),
            ),
        };
        Self {
            uri: file_path_to_uri(&path),
            configured_uri: file_path_to_uri(&path),
            configured_path: Some(path.clone()),
            path: Some(path),
            kind,
            active_version: None,
            summary,
            schema,
            source,
            diagnostics,
            text: Some(text.to_owned()),
        }
    }

    pub(crate) fn overlay_for_open(&self, uri: Uri, text: &str, version: i32) -> Self {
        let Some(path) = self.path.clone() else {
            return self.clone();
        };
        let mut overlay = Self::from_text(path, self.kind, text);
        overlay.uri = Some(uri);
        overlay.active_version = Some(version);
        overlay.configured_uri = self.configured_uri.clone();
        overlay.configured_path = self.configured_path.clone();
        overlay
    }

    pub(crate) fn overlay_for_documents(&self, documents: &OpenDocumentStore) -> Option<Self> {
        let mut matches = documents
            .documents()
            .filter(|document| self.matches_uri(&document.identity().uri))
            .map(|document| {
                (
                    document.identity().uri.clone(),
                    document.text(),
                    document.version(),
                )
            });
        let first = matches.next()?;
        if matches.next().is_some() {
            return None;
        }
        Some(self.overlay_for_open(first.0, first.1, first.2))
    }

    pub(crate) fn has_open_match(&self, documents: &OpenDocumentStore) -> bool {
        documents
            .documents()
            .any(|document| self.matches_uri(&document.identity().uri))
    }

    pub(crate) fn unavailable_overlay(&self, uri: Uri) -> Self {
        let mut overlay = self.clone();
        overlay.uri = Some(uri);
        overlay.active_version = None;
        overlay.schema = None;
        overlay.source = None;
        overlay.summary = None;
        overlay.text = None;
        overlay.diagnostics = schema_unavailable_diagnostic(
            overlay
                .path
                .as_ref()
                .map_or_else(|| "schema".to_owned(), |path| path.display().to_string()),
        );
        overlay
    }

    pub(crate) fn base(&self) -> Self {
        Self::load(self.configured_path.clone())
    }

    pub(crate) fn summary(&self) -> Option<&SchemaSummary> {
        self.summary.as_ref()
    }

    pub(crate) fn schema(&self) -> Option<&ProjectSchema> {
        self.schema.as_ref()
    }

    pub(crate) fn matches_uri(&self, uri: &Uri) -> bool {
        self.uri
            .as_ref()
            .is_some_and(|schema_uri| schema_uri == uri)
            || self
                .configured_uri
                .as_ref()
                .is_some_and(|schema_uri| schema_uri == uri)
            || self.path_matches_uri(uri)
    }

    pub(crate) fn code_action_document(&self) -> Option<crate::features::SchemaCodeActionDocument> {
        Some(crate::features::SchemaCodeActionDocument {
            uri: self.uri.clone()?,
            text: self.text.clone()?,
            summary: self.summary()?.clone(),
            source: self.source.clone()?,
            version: self.active_version?,
        })
    }
}
