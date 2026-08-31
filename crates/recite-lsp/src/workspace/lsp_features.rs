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
        let partition = self.partition_id_for_open(document)?;
        let kernel = self.partition(&partition)?.kernel.snapshot();
        features::completion(
            document.text(),
            position,
            key.as_ref(),
            kernel,
            self.effective_schema_for_partition(&partition)
                .as_ref()
                .and_then(|schema| schema.summary()),
            &self.ui_catalog,
        )
    }

    pub(crate) fn hover(&self, uri: &Uri, position: Position) -> Option<Hover> {
        let document = self.documents.document(uri)?;
        let key = document_key_for_open(document)?;
        let partition = self.partition_id_for_open(document)?;
        let kernel = self.partition(&partition)?.kernel.snapshot();
        features::hover(
            document.text(),
            position,
            &key,
            kernel,
            self.effective_schema_for_partition(&partition)
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
        let (partition, key, text) = self.source_document(uri)?;
        let position = lsp_position_to_source(text, position)?;
        let partition_state = self.partition(&partition)?;
        let documents = self.navigation_documents(&partition);
        features::definition(
            &key,
            position,
            partition_state.kernel.snapshot(),
            &documents,
        )
    }

    pub(crate) fn references(
        &self,
        uri: &Uri,
        position: Position,
        include_declaration: bool,
    ) -> Option<Vec<Location>> {
        let (partition, key, text) = self.source_document(uri)?;
        let position = lsp_position_to_source(text, position)?;
        let partition_state = self.partition(&partition)?;
        let documents = self.navigation_documents(&partition);
        features::references(
            &key,
            position,
            include_declaration,
            partition_state.kernel.snapshot(),
            &documents,
        )
    }

    pub(crate) fn prepare_rename(
        &self,
        uri: &Uri,
        position: Position,
    ) -> Option<PrepareRenameResponse> {
        let (partition, key, text) = self.source_document(uri)?;
        let position = lsp_position_to_source(text, position)?;
        features::prepare_rename(
            &key,
            position,
            self.partition(&partition)?.kernel.snapshot(),
        )
    }

    pub(crate) fn rename(
        &self,
        uri: &Uri,
        position: Position,
        new_name: &str,
    ) -> Option<WorkspaceEdit> {
        let (partition, key, text) = self.source_document(uri)?;
        let position = lsp_position_to_source(text, position)?;
        let partition_state = self.partition(&partition)?;
        let documents = self.navigation_documents(&partition);
        features::rename(
            &key,
            position,
            new_name,
            partition_state.kernel.snapshot(),
            &documents,
        )
    }

    pub(crate) fn code_action(&self, params: &CodeActionParams) -> Option<CodeActionResponse> {
        let partition = self.partition_id_for_uri(&params.text_document.uri)?;
        let partition_state = self.partition(&partition)?;
        let documents = self.code_action_documents(&partition);
        // Schema edits are only safe against an unambiguous, versioned open
        // TOML owner.  Closed/saved evidence has no protocol precondition.
        let schema = self.effective_schema_for_partition(&partition);
        let schema_summary = schema.as_ref().and_then(|schema| schema.summary());
        let schema_document = schema
            .as_ref()
            .and_then(|schema| schema.code_action_document());
        features::code_action(
            params,
            partition_state.kernel.snapshot(),
            &documents,
            schema_document,
            schema_summary,
            &self.ui_catalog,
        )
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

    fn navigation_documents(&self, partition: &str) -> Vec<features::NavigationDocument<'_>> {
        let Some(snapshot) = self.partition(partition).map(|p| p.kernel.snapshot()) else {
            return Vec::new();
        };
        snapshot
            .documents()
            .iter()
            .filter_map(|document| {
                let uri = self.uri_for_document_key(partition, document.key())?;
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

    fn source_document(&self, uri: &Uri) -> Option<(String, DocumentKey, &str)> {
        if let Some(document) = self.documents.document(uri) {
            return Some((
                self.partition_id_for_open(document)?,
                document_key_for_open(document)?,
                document.text(),
            ));
        }
        let document = self.saved.document_by_uri(uri)?;
        Some((
            self.partition_id_for_saved(document)?,
            document_key_for_saved(document)?,
            document.text.as_str(),
        ))
    }

    fn uri_for_document_key(&self, partition: &str, key: &DocumentKey) -> Option<&Uri> {
        let document = self.partition(partition)?.kernel.snapshot().document(key)?;
        match document.layer() {
            DocumentLayer::Open => self.partition(partition)?.open_owners.get(key),
            DocumentLayer::Saved => self
                .saved
                .documents
                .values()
                .find(|document| {
                    self.partition_id_for_saved(document).as_deref() == Some(partition)
                        && document_key_for_saved(document).as_ref() == Some(key)
                })
                .map(|document| &document.identity.uri),
            _ => None,
        }
    }

    fn code_action_documents(&self, partition: &str) -> Vec<features::CodeActionDocument<'_>> {
        let Some(compiler_snapshot) = self.partition(partition).map(|p| p.kernel.snapshot()) else {
            return Vec::new();
        };
        self.snapshot
            .summaries()
            .iter()
            .filter_map(|summary| {
                if self.partition_id_for_uri(summary.uri()).as_deref() != Some(partition)
                    || self.is_schema_document_uri(summary.uri())
                {
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
