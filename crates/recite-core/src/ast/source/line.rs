use crate::{LineId, SourceAnchor, SourceId, SourceSpan, SpeakerId};

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
    pub source_id: SourceId,
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
            source_id: id
                .as_ref()
                .and_then(|id| SourceAnchor::new(id.as_str()).ok())
                .and_then(|anchor| SourceId::frozen("line", anchor))
                .unwrap_or(SourceId::Missing),
            id,
            speaker: None,
            source_text,
            metadata: SourceMetadata::new(),
            statements: Vec::new(),
            span,
        }
    }

    #[must_use]
    pub fn with_source_id(mut self, source_id: SourceId) -> Self {
        self.id = source_id.canonical_line_id();
        self.source_id = source_id;
        self
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
