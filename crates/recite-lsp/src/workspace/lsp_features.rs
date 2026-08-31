use lsp_types::{
    CodeActionParams, CodeActionResponse, CompletionResponse, GotoDefinitionResponse, Hover,
    Location, Position, PrepareRenameResponse, Uri, WorkspaceEdit,
};
use recite_compiler::DocumentLayer;
use recite_core::DocumentKey;

use super::{
    LspWorkspace, document_key_for_identity, document_key_for_open, document_key_for_saved,
};
use crate::documents::OpenDocument;
use crate::edit_projection::EditDocument;
use crate::features;
use crate::position::lsp_position_to_source;
use crate::summary::FileSummary;

impl LspWorkspace {
    pub(crate) fn completion(&self, uri: &Uri, position: Position) -> Option<CompletionResponse> {
        let document = self.documents.document(uri)?;
        let key = document_key_for_open(document);
        features::completion(
            document.text(),
            position,
            key.as_ref(),
            self.kernel.snapshot(),
            self.effective_schema()
                .as_ref()
                .and_then(|schema| schema.summary()),
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
            self.effective_schema()
                .as_ref()
                .and_then(|schema| schema.summary()),
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
        // Schema edits are only safe against an unambiguous, versioned open
        // TOML owner.  Closed/saved evidence has no protocol precondition.
        let schema = self.effective_schema();
        let schema_summary = schema.as_ref().and_then(|schema| schema.summary());
        let schema_document = schema
            .as_ref()
            .and_then(|schema| schema.code_action_document());
        features::code_action(
            params,
            self.kernel.snapshot(),
            &documents,
            schema_document,
            schema_summary,
            &self.ui_catalog,
        )
    }

    fn effective_schema(&self) -> Option<super::schema_index::SchemaIndex> {
        if let Some(schema) = self.schema.overlay_for_documents(&self.documents) {
            return Some(schema);
        }
        if self.schema.has_open_match(&self.documents) {
            return None;
        }
        Some(self.schema.clone())
    }

    pub(crate) fn open_document_diagnostics_except(
        &self,
        exclude: Option<&Uri>,
    ) -> Vec<super::DiagnosticRefresh> {
        self.documents
            .documents()
            .filter(|document| !self.is_schema_document_uri(&document.identity().uri))
            .filter(|document| match exclude {
                Some(uri) => document.identity().uri != *uri,
                None => true,
            })
            .map(|document| self.publish_open_document(document))
            .collect()
    }

    fn navigation_documents(&self) -> Vec<features::NavigationDocument<'_>> {
        let snapshot = self.kernel.snapshot();
        snapshot
            .documents()
            .iter()
            .filter_map(|document| {
                let uri = self.uri_for_document_key(document.key())?;
                Some(features::NavigationDocument {
                    uri,
                    key: document.key(),
                    text: document.source_text(),
                    layer: document.layer(),
                    version: document.version(),
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
        let compiler_snapshot = self.kernel.snapshot();
        self.snapshot
            .summaries()
            .iter()
            .filter_map(|summary| {
                if self.is_schema_document_uri(summary.uri()) {
                    return None;
                }
                let key = document_key_for_identity(&summary.identity)?;
                let source = compiler_snapshot.document(&key)?;
                let text = self.text_for_summary(summary)?;
                if text != source.source_text() {
                    return None;
                }
                Some(features::CodeActionDocument {
                    source: EditDocument {
                        key: source.key(),
                        uri: summary.uri(),
                        text: source.source_text(),
                        layer: source.layer(),
                        version: source.version(),
                    },
                    summary,
                })
            })
            .collect()
    }

    pub(super) fn text_for_summary(&self, summary: &FileSummary) -> Option<&str> {
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
