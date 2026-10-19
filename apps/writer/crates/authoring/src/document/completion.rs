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
    pub(super) source: Arc<str>,
    pub(super) key: DocumentKey,
    pub(super) version: i64,
    pub(super) kernel: Arc<AuthoringKernel>,
    pub(super) context: Arc<ProjectContext>,
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
impl Document {
    pub(crate) fn navigation_analysis_is_current(
        &self,
        base: &CompletionSnapshot,
        candidate: &Document,
    ) -> bool {
        base.is_current(self)
            && self.key == candidate.key
            && Arc::ptr_eq(&self.source, &candidate.source)
            && (candidate.version == self.version
                || Some(candidate.version) == self.version.checked_add(1))
    }
    pub(crate) fn accept_navigation_analysis(
        &mut self,
        base: &CompletionSnapshot,
        candidate: &mut Document,
    ) -> Result<(), EditError> {
        if !self.navigation_analysis_is_current(base, candidate) {
            return Err(EditError::Stale);
        }
        std::mem::swap(&mut self.kernel, &mut candidate.kernel);
        std::mem::swap(&mut self.context, &mut candidate.context);
        std::mem::swap(&mut self.projections, &mut candidate.projections);
        std::mem::swap(&mut self.version, &mut candidate.version);
        Ok(())
    }
}
impl CompletionSnapshot {
    pub(crate) fn rules_revision(&self, candidate: &Document) -> Option<i64> {
        (self.key == candidate.key
            && Arc::ptr_eq(&self.source, &candidate.source)
            && self.context.schema == candidate.context.schema)
            .then_some(candidate.version)
    }
    /// Open a sibling from captured analysis without borrowing a live UI document.
    pub fn open_sibling_with_control(
        &self,
        key: DocumentKey,
        source: impl Into<String>,
        context: ProjectContext,
        control: &dyn WorkControl,
    ) -> Result<Document, EditError> {
        self.projection_document()
            .open_sibling_with_control(key, source, context, control)
    }

    pub(crate) fn projection_document(&self) -> Document {
        Document {
            source: self.source.clone(),
            key: self.key.clone(),
            version: self.version,
            kernel: self.kernel.clone(),
            context: self.context.clone(),
            history: crate::history::History::default(),
            projections: crate::projection_cache::ProjectionCache::default(),
        }
    }

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
