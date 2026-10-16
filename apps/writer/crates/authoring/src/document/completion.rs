//! A completion query owns immutable analysis, never a document's history or projections.
use super::{Document, EditError, ProjectContext};
use crate::SourceCompletions;
use recite_compiler::authoring::{
    AuthoringError, AuthoringKernel, AuthoringRequest, DocumentVersion, OpenDocument, QueryResult,
    WorkControl,
};
use recite_core::{DocumentKey, SourcePosition};
use std::sync::Arc;

#[derive(Clone)]
pub struct CompletionSnapshot {
    source: Arc<str>,
    key: DocumentKey,
    version: i64,
    kernel: Arc<AuthoringKernel>,
    context: Arc<ProjectContext>,
}
impl Document {
    pub fn completion_snapshot(&self) -> CompletionSnapshot {
        CompletionSnapshot {
            source: Arc::clone(&self.source),
            key: self.key.clone(),
            version: self.version,
            kernel: Arc::clone(&self.kernel),
            context: Arc::clone(&self.context),
        }
    }
}
impl CompletionSnapshot {
    /// Identity covers schema and saved inputs even when accepted bytes did not change.
    pub fn is_current(&self, document: &Document) -> bool {
        self.version == document.version
            && self.key == document.key
            && Arc::ptr_eq(&self.source, &document.source)
            && Arc::ptr_eq(&self.kernel, &document.kernel)
            && Arc::ptr_eq(&self.context, &document.context)
    }

    pub fn same_analysis(&self, other: &Self) -> bool {
        self.version == other.version
            && self.key == other.key
            && Arc::ptr_eq(&self.source, &other.source)
            && Arc::ptr_eq(&self.kernel, &other.kernel)
            && Arc::ptr_eq(&self.context, &other.context)
    }

    pub fn complete(
        &self,
        draft: &str,
        position: SourcePosition,
        control: &dyn WorkControl,
    ) -> Result<SourceCompletions, EditError> {
        control.checkpoint().map_err(AuthoringError::from)?;
        let accepted = draft == self.source.as_ref();
        let source = if accepted {
            Arc::clone(&self.source)
        } else {
            Arc::from(draft)
        };
        let updated;
        let fallback;
        let snapshot = if accepted {
            self.kernel.snapshot()
        } else if let Some(version) = self.version.checked_add(1) {
            updated = self.kernel.updated(
                AuthoringRequest::new(
                    self.kernel.snapshot().generation(),
                    self.context.documents.clone(),
                    [OpenDocument::from_shared(
                        self.key.clone(),
                        DocumentVersion::new(version),
                        source,
                    )],
                ),
                control,
            )?;
            updated.snapshot()
        } else {
            fallback = Document::in_project_with_control(
                self.key.clone(),
                draft,
                self.context.as_ref().clone(),
                control,
            )?;
            fallback.kernel.snapshot()
        };
        control.checkpoint().map_err(AuthoringError::from)?;
        let candidates = match snapshot.complete(&self.key, position) {
            QueryResult::Ready(value) | QueryResult::Partial { value, .. } => value,
            _ => Vec::new(),
        };
        control.checkpoint().map_err(AuthoringError::from)?;
        let source = snapshot
            .document(&self.key)
            .ok_or(EditError::Position)?
            .source_index()
            .clone();
        Ok(SourceCompletions { source, candidates })
    }
}
