use recite_compiler::{
    AuthoringError, AuthoringKernel, AuthoringRequest, DocumentVersion, OpenDocument, SavedDocument,
};
use recite_core::{CoreValueError, Diagnostic, DocumentKey, ProjectSchema};

use crate::{DOCUMENT_NAME, Passage, projection};

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
    #[error("This content needs Source view in this experiment: {0}")]
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

/// One in-memory document; candidates share edits and validation, not widget state.
pub struct Document {
    source: String,
    key: DocumentKey,
    version: i64,
    kernel: AuthoringKernel,
    context: ProjectContext,
    undo: Vec<String>,
    redo: Vec<String>,
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
            source: String::new(),
            key,
            version: 0,
            kernel,
            context,
            undo: Vec::new(),
            redo: Vec::new(),
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
                self.source.clone(),
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
    pub const fn revision(&self) -> i64 {
        self.version
    }
    pub fn passages(&self) -> Result<Vec<Passage>, EditError> {
        projection::passages(&self.source)
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
        if source == self.source {
            return Ok(());
        }
        let previous = self.source.clone();
        self.accept(source)?;
        self.undo.push(previous);
        self.redo.clear();
        Ok(())
    }

    pub fn undo(&mut self) -> Result<bool, EditError> {
        let Some(previous) = self.undo.last().cloned() else {
            return Ok(false);
        };
        let current = self.source.clone();
        self.accept(previous)?;
        self.undo.pop();
        self.redo.push(current);
        Ok(true)
    }

    pub fn redo(&mut self) -> Result<bool, EditError> {
        let Some(next) = self.redo.last().cloned() else {
            return Ok(false);
        };
        let current = self.source.clone();
        self.accept(next)?;
        self.redo.pop();
        self.undo.push(current);
        Ok(true)
    }

    pub(crate) fn check_revision(&self, expected: i64) -> Result<(), EditError> {
        if expected != self.version {
            return Err(EditError::Stale);
        }
        Ok(())
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
        self.source = source;
        self.version = version;
        Ok(())
    }
}
