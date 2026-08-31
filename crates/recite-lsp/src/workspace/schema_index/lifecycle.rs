use std::fs;

use lsp_types::Uri;

use super::SchemaIndex;
use crate::paths::stable_path_identity;
use crate::paths::uri_to_file_path;
use crate::workspace::{DiagnosticRefresh, DocumentDiagnostics, SnapshotGeneration};

impl SchemaIndex {
    pub(crate) fn needs_refresh(&self) -> bool {
        !self.diagnostics.is_empty() || self.active_version.is_some()
    }

    pub(crate) fn configured_path(&self) -> Option<&std::path::Path> {
        self.configured_path.as_deref()
    }

    pub(crate) fn target_identity(&self) -> Option<String> {
        self.path
            .as_deref()
            .or(self.configured_path.as_deref())
            .map(stable_path_identity)
    }

    pub(crate) fn canonical_uri(&self) -> Option<Uri> {
        self.path
            .as_deref()
            .and_then(crate::paths::file_path_to_uri)
    }

    pub(crate) fn clear_refresh(
        &self,
        generation: SnapshotGeneration,
    ) -> Option<DiagnosticRefresh> {
        if self.diagnostics.is_empty() {
            return None;
        }
        let uri = self.uri.clone()?;
        if self.active_version.is_some() {
            return Some(DiagnosticRefresh::Publish(DocumentDiagnostics {
                uri,
                text: self.text.clone().unwrap_or_default(),
                version: self.active_version,
                diagnostics: Vec::new(),
                generation,
            }));
        }
        Some(DiagnosticRefresh::Clear {
            uri,
            version: None,
            generation,
        })
    }

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
            let uri = self.uri.clone()?;
            if self.active_version.is_some() {
                // A live overlay owns a protocol version even when it has
                // become valid. Publish the empty set with that version so a
                // client cannot mistake this clear for the saved disk state.
                Some(DiagnosticRefresh::Publish(DocumentDiagnostics {
                    uri,
                    text: self.text.clone().unwrap_or_default(),
                    version: self.active_version,
                    diagnostics: Vec::new(),
                    generation,
                }))
            } else {
                Some(DiagnosticRefresh::Clear {
                    uri,
                    version: None,
                    generation,
                })
            }
        })
    }
}
