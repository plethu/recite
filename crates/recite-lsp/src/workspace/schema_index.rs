use std::fs;
use std::path::PathBuf;

use lsp_types::Uri;
use recite_core::{
    Diagnostic, DiagnosticArgumentValue, DiagnosticCode, DiagnosticPresentationId, ProjectSchema,
    SourcePosition, SourceSpan, contract_for, load_schema_manifest_str,
};

use super::{DiagnosticRefresh, DocumentDiagnostics, SnapshotGeneration};
use crate::paths::{file_path_to_uri, uri_to_file_path};
use crate::summary::SchemaSummary;

const SCHEMA_LOAD_ERROR: DiagnosticCode = DiagnosticCode::new_static("RECITE_SCHEMA001");

#[derive(Clone)]
pub(crate) struct SchemaIndex {
    uri: Option<Uri>,
    path: Option<PathBuf>,
    #[allow(dead_code)]
    summary: Option<SchemaSummary>,
    schema: Option<ProjectSchema>,
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
                diagnostics: Vec::new(),
                text: None,
            };
        };
        let path = fs::canonicalize(&path).unwrap_or(path);
        let uri = file_path_to_uri(&path);
        let display_path = path.display().to_string();
        let source = match fs::read_to_string(&path) {
            Ok(source) => source,
            Err(error) => {
                return Self {
                    uri,
                    path: Some(path),
                    summary: None,
                    schema: None,
                    diagnostics: schema_io_diagnostic(display_path, &error),
                    text: None,
                };
            }
        };

        let report = load_schema_manifest_str(display_path, &source);
        let summary = report.schema.as_ref().map(SchemaSummary::from_schema);
        Self {
            uri,
            path: Some(path),
            summary,
            schema: report.schema,
            diagnostics: report.diagnostics,
            text: Some(source),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn summary(&self) -> Option<&SchemaSummary> {
        self.summary.as_ref()
    }

    pub(crate) fn schema(&self) -> Option<&ProjectSchema> {
        self.schema.as_ref()
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
    ) -> Option<crate::features::SchemaCodeActionDocument<'_>> {
        Some(crate::features::SchemaCodeActionDocument {
            uri: self.uri.as_ref()?,
            text: self.text.as_deref()?,
            summary: self.summary.as_ref()?,
            version: None,
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
