use recite_compiler::{AuthoringRequest, OpenDocument as KernelOpenDocument};

use super::LspWorkspace;
use super::document_keys::standalone_document_key;
use crate::documents::OpenDocument;

impl LspWorkspace {
    pub(super) fn standalone_open_diagnostics(
        &self,
        document: &OpenDocument,
    ) -> Vec<recite_core::Diagnostic> {
        if self.is_schema_document_uri(&document.identity().uri) {
            return Vec::new();
        }
        let Some(key) = standalone_document_key(document) else {
            return Vec::new();
        };
        let mut kernel = self.new_kernel();
        let request = AuthoringRequest::new(
            kernel.snapshot().generation(),
            Vec::new(),
            vec![KernelOpenDocument::new(
                key.clone(),
                recite_compiler::DocumentVersion::new(i64::from(document.version())),
                document.text().to_owned(),
            )],
        )
        .with_project_completeness(self.saved.partition_is_complete("standalone"));
        kernel
            .apply(request)
            .ok()
            .and_then(|_| kernel.snapshot().document(&key))
            .map_or_else(Vec::new, |document| document.diagnostics().to_vec())
    }
}
