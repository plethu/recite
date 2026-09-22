use recite_compiler::{
    AuthoringError, AuthoringKernel, AuthoringRequest, DocumentVersion, OpenDocument, SavedDocument,
};
use recite_core::{CoreValueError, Diagnostic, DocumentKey, ProjectSchema};

use crate::{DOCUMENT_NAME, Passage};

#[derive(Debug, thiserror::Error)]
pub enum EditError {
    #[error(transparent)]
    DocumentKey(#[from] recite_core::DocumentKeyError),
    #[error(transparent)]
    Value(#[from] CoreValueError),
    #[error(transparent)]
    Authoring(#[from] AuthoringError),
    #[error(transparent)]
    Plan(#[from] recite_compiler::AuthoringEditError),
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
    #[error("The selected passage no longer exists or has no unique frozen ID.")]
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
    kernel: AuthoringKernel,
    context: ProjectContext,
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
        let kernel = context
            .schema
            .clone()
            .map_or_else(AuthoringKernel::new, AuthoringKernel::with_schema);
        let mut document = Self {
            source: String::new().into(),
            key,
            version: 0,
            kernel,
            context,
            history: crate::history::History::default(),
            projections: crate::projection_cache::ProjectionCache::default(),
        };
        document.accept(source.into())?;
        Ok(document)
    }

    pub fn refresh_project(&mut self, context: ProjectContext) -> Result<(), EditError> {
        let version = self
            .version
            .checked_add(1)
            .ok_or(EditError::RevisionExhausted)?;
        let mut kernel = context
            .schema
            .clone()
            .map_or_else(AuthoringKernel::new, AuthoringKernel::with_schema);
        kernel.apply(AuthoringRequest::new(
            kernel.snapshot().generation(),
            context.documents.clone(),
            [OpenDocument::new(
                self.key.clone(),
                DocumentVersion::new(version),
                self.source.to_string(),
            )],
        ))?;
        self.kernel = kernel;
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
        self.kernel
            .snapshot()
            .documents()
            .iter()
            .filter(|document| document.key() == &self.key)
            .flat_map(|document| document.summary().blocks())
            .map(|block| block.id().as_str().to_owned())
            .collect()
    }

    /// Source editing preserves invalid buffers so the writer can repair them.
    pub fn replace_source(&mut self, expected: i64, source: String) -> Result<(), EditError> {
        self.check_revision(expected)?;
        if source == self.source.as_ref() {
            return Ok(());
        }
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
        self.accept(source)?;
        self.history.did_undo();
        Ok(true)
    }
    pub fn redo(&mut self) -> Result<bool, EditError> {
        let Some(source) = self.history.redo_source(&self.source) else {
            return Ok(false);
        };
        self.accept(source)?;
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
        self.context.clone()
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

    fn accept(&mut self, source: String) -> Result<(), EditError> {
        let version = self
            .version
            .checked_add(1)
            .ok_or(EditError::RevisionExhausted)?;
        self.kernel.apply(AuthoringRequest::new(
            self.kernel.snapshot().generation(),
            self.context.documents.clone(),
            [OpenDocument::new(
                self.key.clone(),
                DocumentVersion::new(version),
                source.clone(),
            )],
        ))?;
        self.projections = crate::projection_cache::ProjectionCache::default();
        self.source = source.into();
        self.version = version;
        Ok(())
    }
}

mod localisation;
