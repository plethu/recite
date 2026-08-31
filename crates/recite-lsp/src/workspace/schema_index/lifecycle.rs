use std::fs;

use lsp_types::Uri;

use super::SchemaIndex;
use crate::paths::uri_to_file_path;
use crate::workspace::{DiagnosticRefresh, DocumentDiagnostics, SnapshotGeneration};

impl SchemaIndex {
    pub(crate) fn diagnostics_refresh(
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
            version: self.active_version,
            diagnostics: self.diagnostics.clone(),
            generation,
        }))
    }

    pub(super) fn path_matches_uri(&self, uri: &Uri) -> bool {
        let Some(schema_path) = &self.path else {
            return false;
        };
        uri_to_file_path(uri)
            .and_then(|path| fs::canonicalize(path).ok())
            .is_some_and(|path| path == *schema_path)
    }

    pub(crate) fn refresh_or_clear(
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
