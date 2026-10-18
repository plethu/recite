//! Source editing separates immutable preparation from checked publication.
use super::{CompletionSnapshot, Document, EditError};
use crate::{
    DraftSnapshot,
    history::Change,
    projection_cache::{CachedProjections, ProjectionCache},
};
use recite_compiler::authoring::{
    AuthoringError, AuthoringKernel, AuthoringRequest, DocumentVersion, OpenDocument, WorkControl,
};
use std::sync::Arc;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[non_exhaustive]
pub enum EditOperation {
    Apply,
    Undo,
    Redo,
}

/// An immutable edit request; capturing it never flattens a Rope or copies history.
pub(crate) struct SourceEditRequest {
    base: CompletionSnapshot,
    pub(crate) draft: DraftSnapshot,
    operation: Operation,
    rules: Option<Box<crate::ReplyRules>>,
}
enum Operation {
    Apply,
    Prose(String, CachedProjections),
    Undo(Arc<Change>),
    Redo(Arc<Change>),
}
enum HistoryEdit {
    None,
    Record(Change),
    Undo,
    Redo,
}

/// Prepared analysis and displaced owners. Return this value to the preparing
/// thread after acceptance so old projections/history are not destroyed by the UI.
pub(crate) struct PreparedSourceEdit {
    pub(crate) request: SourceEditRequest,
    pub(crate) source: Arc<str>,
    kernel: Option<Arc<AuthoringKernel>>,
    history: HistoryEdit,
    retired_projections: Option<ProjectionCache>,
    projections: Option<ProjectionCache>,
    retired_history: Vec<Arc<Change>>,
    accepted: bool,
}

impl Document {
    pub(crate) fn source_edit_request(
        &self,
        draft: DraftSnapshot,
        operation: EditOperation,
    ) -> Option<SourceEditRequest> {
        let operation = match operation {
            EditOperation::Apply => Operation::Apply,

            EditOperation::Undo => Operation::Undo(self.history.undo_change()?),
            EditOperation::Redo => Operation::Redo(self.history.redo_change()?),
        };
        Some(SourceEditRequest {
            base: self.completion_snapshot(),
            draft,
            operation,
            rules: None,
        })
    }

    pub(crate) fn prose_edit_request(&self, draft: DraftSnapshot, id: String) -> SourceEditRequest {
        SourceEditRequest {
            base: self.completion_snapshot(),
            draft,
            operation: Operation::Prose(id, self.projections.capture_local()),
            rules: None,
        }
    }

    pub(crate) fn accept_source_edit(
        &mut self,
        prepared: &mut PreparedSourceEdit,
    ) -> Result<(), EditError> {
        if prepared.accepted || !prepared.request.base.is_current(self) {
            return Err(EditError::Stale);
        }
        if let Some(kernel) = prepared.kernel.take() {
            self.kernel = kernel;
            prepared.retired_projections = Some(std::mem::take(&mut self.projections));
            if let Some(projections) = prepared.projections.take() {
                self.projections = projections;
            }
            self.source = prepared.source.clone();
            self.version += 1; // Preparation checked overflow against this exact revision.
        }
        match std::mem::replace(&mut prepared.history, HistoryEdit::None) {
            HistoryEdit::None => {}
            HistoryEdit::Record(change) => {
                prepared.retired_history = self.history.record_retained(change)
            }
            HistoryEdit::Undo => self.history.did_undo(),
            HistoryEdit::Redo => self.history.did_redo(),
        }
        prepared.accepted = true;
        Ok(())
    }
}

impl SourceEditRequest {
    /// Preserve Rules validation when its controls still describe this exact draft.
    pub(crate) fn with_rules(mut self, rules: crate::ReplyRules) -> Self {
        self.rules = Some(Box::new(rules));
        self
    }

    pub(crate) fn prepare(
        self,
        control: &dyn WorkControl,
    ) -> Result<PreparedSourceEdit, EditError> {
        control.checkpoint().map_err(AuthoringError::from)?;
        let mut projections = None;
        let mut source: Arc<str> = match &self.operation {
            Operation::Apply => self.draft.text_snapshot(),
            Operation::Prose(id, cached) => {
                let mut document = self.base.projection_document();
                document.projections = cached.restore();
                let (source, prepared) = document.prepare_text(id, self.draft.as_str())?;
                projections = prepared;
                source
            }
            Operation::Undo(change) => change.apply(&self.base.source, true).into(),
            Operation::Redo(change) => change.apply(&self.base.source, false).into(),
        };
        if let Some(rules) = &self.rules
            && rules.belongs_to(&self.base.projection_document())
            && rules.changed()
            && rules
                .source()
                .is_ok_and(|rules_source| rules_source.as_str() == source.as_ref())
        {
            rules.validate(&self.base.projection_document())?;
        }
        let (kernel, history) = if source == self.base.source {
            // A pristine Rope may have flattened into a second whole-file string.
            source = self.base.source.clone();
            (None, HistoryEdit::None)
        } else {
            let version = self
                .base
                .version
                .checked_add(1)
                .ok_or(EditError::RevisionExhausted)?;
            let history = match &self.operation {
                Operation::Apply | Operation::Prose(..) => {
                    HistoryEdit::Record(Change::between(&self.base.source, &source))
                }
                Operation::Undo(_) => HistoryEdit::Undo,
                Operation::Redo(_) => HistoryEdit::Redo,
            };
            let kernel = self.base.kernel.updated(
                AuthoringRequest::new(
                    self.base.kernel.snapshot().generation(),
                    self.base.context.documents.clone(),
                    [OpenDocument::from_shared(
                        self.base.key.clone(),
                        DocumentVersion::new(version),
                        source.clone(),
                    )],
                ),
                control,
            )?;
            (Some(Arc::new(kernel)), history)
        };
        control.checkpoint().map_err(AuthoringError::from)?;
        Ok(PreparedSourceEdit {
            request: self,
            source,
            kernel,
            history,
            retired_projections: None,
            projections,
            retired_history: Vec::new(),
            accepted: false,
        })
    }
}

impl PreparedSourceEdit {
    pub(crate) fn projection_document(&self) -> Document {
        let mut document = self.request.base.projection_document();
        if let Some(kernel) = &self.kernel {
            document.source = self.source.clone();
            document.kernel = kernel.clone();
            document.version += 1; // Preparation checked this exact revision.
            if let Some(projections) = &self.projections {
                document.projections = projections.capture_local().restore();
            }
        }
        document
    }
}
