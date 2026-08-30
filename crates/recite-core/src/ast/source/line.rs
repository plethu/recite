use crate::{LineId, SourceAnchor, SourceId, SourceSpan, SpeakerId};

use super::{InterpolationBinding, SourceMetadata, Statement};

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
    pub source_id_span: Option<SourceSpan>,
    pub source_id_insertion_span: SourceSpan,
    pub id: Option<LineId>,
    pub speaker: Option<SpeakerId>,
    pub source_text: SourceText,
    /// Optional second source form selected by gettext plural rules.
    pub plural_source_text: Option<SourceText>,
    pub interpolation_bindings: Vec<InterpolationBinding>,
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
            source_id_span: None,
            source_id_insertion_span: span.clone(),
            id,
            speaker: None,
            source_text,
            plural_source_text: None,
            interpolation_bindings: Vec::new(),
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
    pub fn with_source_id_spans(
        mut self,
        source_id_span: Option<SourceSpan>,
        source_id_insertion_span: SourceSpan,
    ) -> Self {
        self.source_id_span = source_id_span;
        self.source_id_insertion_span = source_id_insertion_span;
        self
    }

    #[must_use]
    pub fn with_plural_source_text(mut self, source_text: SourceText) -> Self {
        self.plural_source_text = Some(source_text);
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
    pub fn with_interpolation_bindings(mut self, bindings: Vec<InterpolationBinding>) -> Self {
        self.interpolation_bindings = bindings;
        self
    }

    #[must_use]
    pub fn with_statements(mut self, statements: Vec<Statement>) -> Self {
        self.statements = statements;
        self
    }
}
