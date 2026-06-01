use std::fs;
use std::path::PathBuf;

use lsp_types::Uri;
use recite_core::{
    Diagnostic, DiagnosticCode, SourcePosition, SourceSpan, load_schema_manifest_str,
};

use super::{DiagnosticRefresh, DocumentDiagnostics, SnapshotGeneration};
use crate::paths::file_path_to_uri;
use crate::summary::SchemaSummary;

const SCHEMA_LOAD_ERROR: DiagnosticCode = DiagnosticCode::new_static("RECITE_SCHEMA001");

pub(crate) struct SchemaIndex {
    uri: Option<Uri>,
    #[allow(dead_code)]
    summary: Option<SchemaSummary>,
    diagnostics: Vec<Diagnostic>,
    text: Option<String>,
}

impl SchemaIndex {
    pub(super) fn load(path: Option<PathBuf>) -> Self {
        let Some(path) = path else {
            return Self {
                uri: None,
                summary: None,
                diagnostics: Vec::new(),
                text: None,
            };
        };
        let uri = file_path_to_uri(&path);
        let display_path = path.display().to_string();
        let source = match fs::read_to_string(&path) {
            Ok(source) => source,
            Err(error) => {
                return Self {
                    uri,
                    summary: None,
                    diagnostics: schema_io_diagnostic(display_path, &error),
                    text: None,
                };
            }
        };

        let report = load_schema_manifest_str(display_path, &source);
        let summary = report.schema.as_ref().map(SchemaSummary::from_schema);
        Self {
            uri,
            summary,
            diagnostics: report.diagnostics,
            text: Some(source),
        }
    }

    #[allow(dead_code)]
    pub(crate) fn summary(&self) -> Option<&SchemaSummary> {
        self.summary.as_ref()
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
}

fn schema_io_diagnostic(file: String, error: &std::io::Error) -> Vec<Diagnostic> {
    let Ok(start) = SourcePosition::new(1, 1) else {
        return Vec::new();
    };
    vec![Diagnostic::error(
        SCHEMA_LOAD_ERROR,
        format!("failed to read schema manifest: {error}"),
        SourceSpan::new(file, start, None),
    )]
}
