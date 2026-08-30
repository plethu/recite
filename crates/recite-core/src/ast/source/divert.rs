use crate::{BlockId, SourceSpan};

/// Source-format token used to target dialogue termination.
pub const END_DIVERT_TARGET: &str = "END";

/// A standalone divert statement.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Divert {
    pub target: DivertTarget,
    pub span: SourceSpan,
}

impl Divert {
    #[must_use]
    pub fn new(target: DivertTarget, span: SourceSpan) -> Self {
        Self { target, span }
    }
}

/// Source-level divert targets. Validation resolves unknown block references.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum DivertTarget {
    Block(BlockReference),
    End,
}

/// A same-file or cross-file block reference.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct BlockReference {
    pub file: Option<String>,
    pub file_span: Option<SourceSpan>,
    pub block_id: BlockId,
    pub block_id_span: Option<SourceSpan>,
}

impl BlockReference {
    #[must_use]
    pub fn local(block_id: BlockId) -> Self {
        Self {
            file: None,
            file_span: None,
            block_id,
            block_id_span: None,
        }
    }

    #[must_use]
    pub fn external(file: impl Into<String>, block_id: BlockId) -> Self {
        Self {
            file: Some(file.into()),
            file_span: None,
            block_id,
            block_id_span: None,
        }
    }

    #[must_use]
    pub fn with_spans(mut self, file_span: Option<SourceSpan>, block_id_span: SourceSpan) -> Self {
        self.file_span = file_span;
        self.block_id_span = Some(block_id_span);
        self
    }
}
