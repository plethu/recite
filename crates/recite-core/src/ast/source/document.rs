use crate::{BlockId, SourceSpan, SpeakerId};

use super::{SourceMetadata, Statement};

/// A parsed Recite source file.
#[derive(Clone, Debug, PartialEq)]
pub struct SourceFile {
    pub path: String,
    pub blocks: Vec<Block>,
}

impl SourceFile {
    #[must_use]
    pub fn new(path: impl Into<String>, blocks: Vec<Block>) -> Self {
        Self {
            path: path.into(),
            blocks,
        }
    }

    pub fn visit_statements_depth_first<'a>(&'a self, visitor: &mut impl FnMut(&'a Statement)) {
        for block in &self.blocks {
            block.visit_statements_depth_first(visitor);
        }
    }
}

/// A named dialogue block.
#[derive(Clone, Debug, PartialEq)]
pub struct Block {
    pub id: BlockId,
    pub id_span: Option<SourceSpan>,
    pub is_default: bool,
    pub default_speaker: Option<SpeakerId>,
    pub metadata: SourceMetadata,
    pub statements: Vec<Statement>,
    pub span: SourceSpan,
}

impl Block {
    #[must_use]
    pub fn new(id: BlockId, statements: Vec<Statement>, span: SourceSpan) -> Self {
        Self {
            id,
            id_span: None,
            is_default: false,
            default_speaker: None,
            metadata: SourceMetadata::new(),
            statements,
            span,
        }
    }

    #[must_use]
    pub fn with_id_span(mut self, id_span: SourceSpan) -> Self {
        self.id_span = Some(id_span);
        self
    }

    #[must_use]
    pub fn with_default(mut self, is_default: bool) -> Self {
        self.is_default = is_default;
        self
    }

    #[must_use]
    pub fn with_default_speaker(mut self, speaker: SpeakerId) -> Self {
        self.default_speaker = Some(speaker);
        self
    }

    #[must_use]
    pub fn with_metadata(mut self, metadata: SourceMetadata) -> Self {
        self.metadata = metadata;
        self
    }

    pub fn visit_statements_depth_first<'a>(&'a self, visitor: &mut impl FnMut(&'a Statement)) {
        for statement in &self.statements {
            statement.visit_depth_first(visitor);
        }
    }
}
