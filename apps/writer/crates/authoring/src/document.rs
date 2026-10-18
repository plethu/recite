use recite_compiler::authoring::{
    AuthoringError, AuthoringKernel, AuthoringRequest, CancellationToken, DocumentVersion,
    OpenDocument, SavedDocument, WorkControl,
};
use recite_core::{CoreValueError, Diagnostic, DocumentKey, schema::ProjectSchema};

use crate::{DOCUMENT_NAME, Passage};
use std::sync::Arc;

mod preview;
pub use preview::{PreparedPreview, PreviewRequest};
mod source_edit;
pub use source_edit::EditOperation;
pub(crate) use source_edit::{PreparedSourceEdit, SourceEditRequest};
mod completion;
pub use completion::CompletionSnapshot;
mod diagnostics;
pub use diagnostics::DiagnosticSnapshot;

#[derive(Debug, thiserror::Error)]
pub enum EditError {
    #[error(transparent)]
    DocumentKey(#[from] recite_core::DocumentKeyError),
    #[error(transparent)]
    Value(#[from] CoreValueError),
    #[error(transparent)]
    Authoring(#[from] AuthoringError),
    #[error(transparent)]
    Plan(#[from] recite_compiler::authoring::AuthoringEditError),
    #[error(
        "This document changed after the editing field was opened. Reload the field before applying it."
    )]
    Stale,
    #[error("{argument} needs a valid {expected} value.")]
    InvalidRuleValue { argument: String, expected: String },
    #[error("Cannot apply rules: {0}")]
    InvalidRules(String),
    #[error("This content needs Source view: {0}")]
    SourceRequired(&'static str),
    #[error("The selected line no longer exists or has no unique frozen ID.")]
    MissingPassage,
    #[error("The source position cannot be mapped safely.")]
    Position,
    #[error("The document revision cannot advance further.")]
    RevisionExhausted,
    #[error(
        "The proposed text would introduce Recite syntax or lose whitespace. It was not applied; use Source view for this content."
    )]
    NotProse,
    #[error("Choose a section in this document or END.")]
    Destination,
}

/// Immutable saved inputs used beneath the current document's unsaved overlay.
#[derive(Clone, Default)]
pub struct ProjectContext {
    pub documents: Vec<SavedDocument>,
    pub schema: Option<ProjectSchema>,
}

/// One source document with revision-checked edits and compiler diagnostics.
pub struct Document {
    source: std::sync::Arc<str>,
    key: DocumentKey,
    version: i64,
    kernel: Arc<AuthoringKernel>,
    context: Arc<ProjectContext>,
    history: crate::history::History,
    pub(crate) projections: crate::projection_cache::ProjectionCache,
}

impl Document {
    pub fn new(source: impl Into<String>) -> Result<Self, EditError> {
        Self::open(DocumentKey::new(DOCUMENT_NAME)?, source)
    }

    pub fn open(key: DocumentKey, source: impl Into<String>) -> Result<Self, EditError> {
        Self::in_project(key, source, ProjectContext::default())
    }

    pub fn in_project(
        key: DocumentKey,
        source: impl Into<String>,
        context: ProjectContext,
    ) -> Result<Self, EditError> {
        Self::in_project_with_control(key, source, context, &CancellationToken::new())
    }

    /// Construct initial analysis cooperatively; interruption returns no document.
    pub fn in_project_with_control(
        key: DocumentKey,
        source: impl Into<String>,
        context: ProjectContext,
        control: &dyn WorkControl,
    ) -> Result<Self, EditError> {
        control.checkpoint().map_err(AuthoringError::from)?;
        let kernel = context
            .schema
            .clone()
            .map_or_else(|| Ok(AuthoringKernel::new()), AuthoringKernel::with_schema)?;
        let mut document = Self {
            source: String::new().into(),
            key,
            version: 0,
            kernel: Arc::new(kernel),
            context: Arc::new(context),
            history: crate::history::History::default(),
            projections: crate::projection_cache::ProjectionCache::default(),
        };
        let source: String = source.into();
        document.accept_with_control(source.into(), control)?;
        Ok(document)
    }

    /// Open an independent document, sharing unchanged project analysis.
    /// The supplied context replaces every saved input; this document's open
    /// overlay, revisions, history and projections remain private to it.
    pub fn open_sibling_with_control(
        &self,
        key: DocumentKey,
        source: impl Into<String>,
        context: ProjectContext,
        control: &dyn WorkControl,
    ) -> Result<Self, EditError> {
        if key == self.key || context.schema != self.context.schema {
            return Self::in_project_with_control(key, source, context, control);
        }
        control.checkpoint().map_err(AuthoringError::from)?;
        let source: std::sync::Arc<str> = source.into().into();
        let version = 1;
        let kernel = self.kernel.updated(
            AuthoringRequest::new(
                self.kernel.snapshot().generation(),
                context.documents.clone(),
                [OpenDocument::from_shared(
                    key.clone(),
                    DocumentVersion::new(version),
                    source.clone(),
                )],
            ),
            control,
        )?;
        control.checkpoint().map_err(AuthoringError::from)?;
        Ok(Self {
            source,
            key,
            version,
            kernel: Arc::new(kernel),
            context: Arc::new(context),
            history: crate::history::History::default(),
            projections: crate::projection_cache::ProjectionCache::default(),
        })
    }

    pub fn refresh_project(&mut self, context: ProjectContext) -> Result<(), EditError> {
        self.refresh_project_with_control(context, &CancellationToken::new())
    }

    /// Refresh transactionally while retaining unchanged compiler analysis.
    pub fn refresh_project_with_control(
        &mut self,
        context: ProjectContext,
        control: &dyn WorkControl,
    ) -> Result<(), EditError> {
        control.checkpoint().map_err(AuthoringError::from)?;
        let same_schema = self.context.schema == context.schema;
        let same_inputs = self.context.documents.len() == context.documents.len()
            && self
                .context
                .documents
                .iter()
                .zip(&context.documents)
                .all(|(old, new)| old.key() == new.key() && (old.key() == &self.key || old == new));
        // The open source shadows its saved slot. Saving it or switching tabs
        // alone must not invalidate the preview or rebuild project analysis.
        if same_schema && same_inputs {
            let context = Arc::new(context);
            control.checkpoint().map_err(AuthoringError::from)?;
            self.context = context;
            return Ok(());
        }
        let version = self
            .version
            .checked_add(1)
            .ok_or(EditError::RevisionExhausted)?;
        let open = OpenDocument::from_shared(
            self.key.clone(),
            DocumentVersion::new(version),
            self.source_snapshot(),
        );
        let kernel = if same_schema {
            self.kernel.updated(
                AuthoringRequest::new(
                    self.kernel.snapshot().generation(),
                    context.documents.clone(),
                    [open],
                ),
                control,
            )?
        } else {
            let kernel = context
                .schema
                .clone()
                .map_or_else(|| Ok(AuthoringKernel::new()), AuthoringKernel::with_schema)?;
            kernel.updated(
                AuthoringRequest::new(
                    kernel.snapshot().generation(),
                    context.documents.clone(),
                    [open],
                ),
                control,
            )?
        };
        let kernel = Arc::new(kernel);
        let context = Arc::new(context);
        control.checkpoint().map_err(AuthoringError::from)?;
        self.kernel = kernel;
        self.projections.invalidate_project();
        self.context = context;
        self.version = version;
        Ok(())
    }

    pub fn source(&self) -> &str {
        &self.source
    }
    pub fn source_snapshot(&self) -> std::sync::Arc<str> {
        self.source.clone()
    }
    pub const fn revision(&self) -> i64 {
        self.version
    }
    pub fn passages(&self) -> Result<Vec<Passage>, EditError> {
        Ok(self.passage_snapshot()?.to_vec())
    }
    pub fn diagnostics(&self) -> Vec<Diagnostic> {
        self.kernel
            .snapshot()
            .diagnostics()
            .iter()
            .cloned()
            .collect()
    }
    pub fn sections(&self) -> Vec<String> {
        self.block_summaries()
            .iter()
            .map(|block| block.id().as_str().to_owned())
            .collect()
    }

    /// Source-order navigation facts already prepared by accepted analysis.
    pub fn block_summaries(&self) -> &[recite_compiler::authoring::BlockDefinitionSummary] {
        self.kernel
            .snapshot()
            .document(&self.key)
            .map(|document| document.summary().blocks())
            .unwrap_or_default()
    }

    /// Shared accepted analysis for reactive consumers; draft changes keep its identity.
    pub fn summary_snapshot(&self) -> Option<Arc<recite_compiler::authoring::AuthoringSummary>> {
        self.kernel
            .snapshot()
            .document(&self.key)
            .map(|document| document.summary_snapshot())
    }

    /// Project navigation uses the accepted compiler summaries, including this
    /// document's open source, rather than reparsing the saved files.
    pub fn project_sections(&self) -> Vec<(String, Vec<String>)> {
        self.kernel
            .snapshot()
            .documents()
            .iter()
            .map(|document| {
                (
                    document.key().to_string(),
                    document
                        .summary()
                        .blocks()
                        .iter()
                        .map(|block| block.id().as_str().to_owned())
                        .collect(),
                )
            })
            .collect()
    }

    /// Source editing preserves invalid buffers so the writer can repair them.
    pub fn replace_source(
        &mut self,
        expected: i64,
        source: impl AsRef<str> + Into<Arc<str>>,
    ) -> Result<(), EditError> {
        self.check_revision(expected)?;
        if source.as_ref() == self.source.as_ref() {
            return Ok(());
        }
        let source = source.into();
        let change = crate::history::Change::between(&self.source, &source);
        self.accept(source)?;
        self.history.record(change);
        Ok(())
    }

    pub fn can_undo(&self) -> bool {
        self.history.can_undo()
    }

    pub fn undo(&mut self) -> Result<bool, EditError> {
        let Some(source) = self.history.undo_source(&self.source) else {
            return Ok(false);
        };
        self.accept(source.into())?;
        self.history.did_undo();
        Ok(true)
    }
    pub fn redo(&mut self) -> Result<bool, EditError> {
        let Some(source) = self.history.redo_source(&self.source) else {
            return Ok(false);
        };
        self.accept(source.into())?;
        self.history.did_redo();
        Ok(true)
    }
    /// Estimated retained undo/redo allocation, excluding the current document.
    pub fn history_bytes(&self) -> usize {
        self.history.bytes()
    }

    pub(crate) fn check_revision(&self, expected: i64) -> Result<(), EditError> {
        if expected != self.version {
            return Err(EditError::Stale);
        }
        Ok(())
    }

    pub(crate) fn project_context(&self) -> ProjectContext {
        self.context.as_ref().clone()
    }

    pub(crate) fn kernel(&self) -> &AuthoringKernel {
        &self.kernel
    }
    pub(crate) fn schema(&self) -> Option<&ProjectSchema> {
        self.context.schema.as_ref()
    }
    pub fn key(&self) -> &DocumentKey {
        &self.key
    }

    fn accept(&mut self, source: Arc<str>) -> Result<(), EditError> {
        self.accept_with_control(source, &CancellationToken::new())
    }

    fn accept_with_control(
        &mut self,
        source: Arc<str>,
        control: &dyn WorkControl,
    ) -> Result<(), EditError> {
        let version = self
            .version
            .checked_add(1)
            .ok_or(EditError::RevisionExhausted)?;
        let kernel = self.kernel.updated(
            AuthoringRequest::new(
                self.kernel.snapshot().generation(),
                self.context.documents.clone(),
                [OpenDocument::from_shared(
                    self.key.clone(),
                    DocumentVersion::new(version),
                    source.clone(),
                )],
            ),
            control,
        )?;
        let kernel = Arc::new(kernel);
        // Preparation is immutable; cancellation must still win before publication.
        control.checkpoint().map_err(AuthoringError::from)?;
        self.kernel = kernel;
        self.projections = crate::projection_cache::ProjectionCache::default();
        self.source = source;
        self.version = version;
        Ok(())
    }
}

mod localisation;

#[cfg(test)]
mod tests;
