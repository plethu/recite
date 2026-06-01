use crate::{LineId, SourceSpan, SpeakerId};

use super::{SourceMetadata, Statement};

/// Localisable source text with its own span.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SourceText {
    pub text: String,
    pub span: SourceSpan,
}

impl SourceText {
    #[must_use]
    pub fn new(text: impl Into<String>, span: SourceSpan) -> Self {
        Self {
            text: text.into(),
            span,
        }
    }
}

/// A localisable dialogue line. Missing IDs are represented for later
/// compiler/LSP validation.
#[derive(Clone, Debug, PartialEq)]
pub struct Line {
    pub id: Option<LineId>,
    pub speaker: Option<SpeakerId>,
    pub source_text: SourceText,
    pub metadata: SourceMetadata,
    pub statements: Vec<Statement>,
    pub span: SourceSpan,
}

impl Line {
    #[must_use]
    pub fn new(id: Option<LineId>, source_text: SourceText, span: SourceSpan) -> Self {
        Self {
            id,
            speaker: None,
            source_text,
            metadata: SourceMetadata::new(),
            statements: Vec::new(),
            span,
        }
    }

    #[must_use]
    pub fn with_speaker(mut self, speaker: SpeakerId) -> Self {
        self.speaker = Some(speaker);
        self
    }

    #[must_use]
    pub fn with_metadata(mut self, metadata: SourceMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    #[must_use]
    pub fn with_statements(mut self, statements: Vec<Statement>) -> Self {
        self.statements = statements;
        self
    }
}
