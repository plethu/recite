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
pub struct BlockReference {
    pub file: Option<String>,
    pub block_id: BlockId,
}

impl BlockReference {
    #[must_use]
    pub fn local(block_id: BlockId) -> Self {
        Self {
            file: None,
            block_id,
        }
    }

    #[must_use]
    pub fn external(file: impl Into<String>, block_id: BlockId) -> Self {
        Self {
            file: Some(file.into()),
            block_id,
        }
    }
}
