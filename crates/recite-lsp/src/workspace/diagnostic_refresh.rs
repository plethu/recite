use super::{DiagnosticRefresh, LspWorkspace, SchemaRefreshOutcome};
use lsp_types::Uri;

impl DiagnosticRefresh {
    pub(crate) fn into_uri(self) -> Uri {
        match self {
            Self::Publish(diagnostics) => diagnostics.uri,
            Self::Clear { uri, .. } => uri,
        }
    }
}

impl LspWorkspace {
    pub(crate) fn open_diagnostic_uris(&self) -> impl Iterator<Item = &Uri> {
        self.documents
            .documents()
            .map(|document| &document.identity().uri)
            .filter(|uri| !self.query_is_schema(uri))
    }

    /// Recompute a publication that was superseded before transport handoff.
    /// In particular, a suppressed close must eventually clear its exact URI.
    pub(crate) fn diagnostic_refresh_for_uri(&self, uri: &Uri) -> DiagnosticRefresh {
        match self.schema_refresh_for_uri(uri) {
            SchemaRefreshOutcome::Silent => {
                // An open alias that lost schema authority must clear its old
                // publication without claiming the current owner's version.
                return DiagnosticRefresh::Clear {
                    uri: uri.clone(),
                    version: None,
                    generation: self.generation,
                };
            }
            SchemaRefreshOutcome::Refreshes(refreshes) => {
                if let Some(refresh) = refreshes.into_iter().find(|refresh| match refresh {
                    DiagnosticRefresh::Publish(diagnostics) => &diagnostics.uri == uri,
                    DiagnosticRefresh::Clear { uri: candidate, .. } => candidate == uri,
                }) {
                    return refresh;
                }
            }
            SchemaRefreshOutcome::NotSchema => {}
        }
        if let Some(document) = self.documents.document(uri) {
            if self.is_schema_document_uri(uri) {
                return DiagnosticRefresh::Clear {
                    uri: uri.clone(),
                    version: Some(document.version()),
                    generation: self.generation,
                };
            }
            return self.publish_open_document(document);
        }
        if let Some(refresh) = self
            .project_diagnostics_all()
            .into_iter()
            .find(|refresh| matches!(refresh, DiagnosticRefresh::Publish(d) if &d.uri == uri))
        {
            return refresh;
        }
        if let Some(document) = self.saved.document_by_uri(uri)
            && &document.identity.uri == uri
        {
            return self.publish_saved_document(document);
        }
        DiagnosticRefresh::Clear {
            uri: uri.clone(),
            version: None,
            generation: self.generation,
        }
    }

    pub(crate) fn open_version(&self, uri: &Uri) -> Option<i32> {
        self.documents
            .document(uri)
            .map(|document| document.version())
    }
}
