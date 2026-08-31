use std::fs;
use std::path::PathBuf;

use lsp_types::Uri;
use recite_core::{
    Diagnostic, DiagnosticArgumentValue, DiagnosticCode, DiagnosticPresentationId, ProjectSchema,
    SchemaSource, SourcePosition, SourceSpan, contract_for, load_schema_manifest_str,
};

use super::{DiagnosticRefresh, DocumentDiagnostics, SnapshotGeneration};
use crate::paths::{file_path_to_uri, uri_to_file_path};
use crate::summary::SchemaSummary;

const SCHEMA_LOAD_ERROR: DiagnosticCode = DiagnosticCode::new_static("RECITE_SCHEMA001");

#[derive(Clone)]
pub(crate) struct SchemaIndex {
    uri: Option<Uri>,
    path: Option<PathBuf>,
    summary: Option<SchemaSummary>,
    schema: Option<ProjectSchema>,
    source: Option<SchemaSource>,
    diagnostics: Vec<Diagnostic>,
    text: Option<String>,
}

impl SchemaIndex {
    pub(super) fn load(path: Option<PathBuf>) -> Self {
        let Some(path) = path else {
            return Self {
                uri: None,
                path: None,
                summary: None,
                schema: None,
                source: None,
                diagnostics: Vec::new(),
                text: None,
            };
        };
        let path = fs::canonicalize(&path).unwrap_or(path);
        let uri = file_path_to_uri(&path);
        let display_path = path.display().to_string();
        let text = match fs::read_to_string(&path) {
            Ok(source) => source,
            Err(error) => {
                return Self {
                    uri,
                    path: Some(path),
                    summary: None,
                    schema: None,
                    source: None,
                    diagnostics: schema_io_diagnostic(display_path, &error),
                    text: None,
                };
            }
        };

        let mut index = Self::from_text(path, &text);
        index.uri = uri;
        index
    }

    fn from_text(path: PathBuf, text: &str) -> Self {
        let display_path = path.display().to_string();
        let (schema, source, summary, diagnostics) =
            match path.extension().and_then(|ext| ext.to_str()) {
                Some("toml") => {
                    let report = SchemaSource::load_str(display_path, text);
                    let summary = report.source.as_ref().map(SchemaSummary::from_source);
                    let schema = report.source.as_ref().map(|source| source.schema().clone());
                    (schema, report.source, summary, report.diagnostics)
                }
                Some("json") => {
                    let report = load_schema_manifest_str(display_path, text);
                    let summary = report.schema.as_ref().map(SchemaSummary::from_schema);
                    (report.schema, None, summary, report.diagnostics)
                }
                _ => (None, None, None, Vec::new()),
            };
        Self {
            uri: file_path_to_uri(&path),
            path: Some(path),
            summary,
            schema,
            source,
            diagnostics,
            text: Some(text.to_owned()),
        }
    }

    pub(crate) fn source_for_text(
        &self,
        text: &str,
    ) -> Option<crate::features::SchemaCodeActionDocument> {
        let path = self.path.clone()?;
        let mut overlay = Self::from_text(path, text);
        overlay.uri = self.uri.clone();
        overlay.code_action_document(None)
    }

    pub(crate) fn summary(&self) -> Option<&SchemaSummary> {
        self.summary.as_ref()
    }

    pub(crate) fn schema(&self) -> Option<&ProjectSchema> {
        self.schema.as_ref()
    }

    pub(crate) fn is_generated(&self) -> bool {
        self.schema.is_some() && self.source.is_none()
    }

    pub(crate) fn uri(&self) -> Option<&Uri> {
        self.uri.as_ref()
    }

    pub(crate) fn matches_uri(&self, uri: &Uri) -> bool {
        self.uri
            .as_ref()
            .is_some_and(|schema_uri| schema_uri == uri)
            || self.path_matches_uri(uri)
    }

    pub(crate) fn path(&self) -> Option<&std::path::Path> {
        self.path.as_deref()
    }

    pub(crate) fn code_action_document(
        &self,
        version: Option<i32>,
    ) -> Option<crate::features::SchemaCodeActionDocument> {
        Some(crate::features::SchemaCodeActionDocument {
            uri: self.uri.clone()?,
            text: self.text.clone()?,
            summary: self.summary()?.clone(),
            source: self.source.clone()?,
            version,
        })
    }

    #[allow(dead_code)]
    pub(crate) fn diagnostics(&self) -> &[Diagnostic] {
        &self.diagnostics
    }

    pub(super) fn diagnostics_refresh(
        &self,
        generation: SnapshotGeneration,
    ) -> Option<DiagnosticRefresh> {
        let uri = self.uri.clone()?;
        if self.diagnostics.is_empty() {
            return None;
        }

        Some(DiagnosticRefresh::Publish(DocumentDiagnostics {
            uri,
            text: self.text.clone().unwrap_or_default(),
            version: None,
            diagnostics: self.diagnostics.clone(),
            generation,
        }))
    }

    pub(super) fn refresh_uri(&mut self, uri: &Uri) -> bool {
        let Some(schema_uri) = &self.uri else {
            return false;
        };
        if schema_uri != uri && !self.path_matches_uri(uri) {
            return false;
        }

        let path = self.path.clone();
        *self = Self::load(path);
        true
    }

    fn path_matches_uri(&self, uri: &Uri) -> bool {
        let Some(schema_path) = &self.path else {
            return false;
        };
        uri_to_file_path(uri)
            .and_then(|path| fs::canonicalize(path).ok())
            .is_some_and(|path| path == *schema_path)
    }

    pub(super) fn refresh_or_clear(
        &self,
        generation: SnapshotGeneration,
    ) -> Option<DiagnosticRefresh> {
        self.diagnostics_refresh(generation).or_else(|| {
            self.uri
                .clone()
                .map(|uri| DiagnosticRefresh::Clear { uri, generation })
        })
    }
}

#[allow(
    clippy::expect_used,
    reason = "the schema read contract is a static first-party registry invariant"
)]
fn schema_io_diagnostic(file: String, error: &std::io::Error) -> Vec<Diagnostic> {
    let Ok(start) = SourcePosition::new(1, 1) else {
        return Vec::new();
    };
    let presentation_id = DiagnosticPresentationId::new_static("diagnostic-schema-001-read");
    let contract = contract_for(&SCHEMA_LOAD_ERROR, &presentation_id)
        .expect("schema read diagnostic contract is registered");
    let diagnostic = Diagnostic::error_from_contract(
        contract,
        format!("failed to read schema manifest: {error}"),
        SourceSpan::new(file, start, None),
        [("detail", DiagnosticArgumentValue::String(error.to_string()))],
    )
    .expect("schema read diagnostic arguments match their contract");
    vec![diagnostic]
}
