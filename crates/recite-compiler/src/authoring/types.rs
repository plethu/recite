use recite_core::{BlockId, SourceSpan};

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct BlockDefinitionSummary {
    pub(super) id: BlockId,
    pub(super) span: SourceSpan,
}

impl BlockDefinitionSummary {
    #[must_use]
    pub fn id(&self) -> &BlockId {
        &self.id
    }
    #[must_use]
    pub fn span(&self) -> &SourceSpan {
        &self.span
    }
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct BlockReferenceSummary {
    pub(super) file: Option<String>,
    pub(super) block_id: BlockId,
    pub(super) span: SourceSpan,
}

impl BlockReferenceSummary {
    #[must_use]
    pub fn file(&self) -> Option<&str> {
        self.file.as_deref()
    }
    #[must_use]
    pub fn block_id(&self) -> &BlockId {
        &self.block_id
    }
    #[must_use]
    pub fn span(&self) -> &SourceSpan {
        &self.span
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum StableIdKind {
    Line,
    Choice,
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct StableIdSummary {
    pub(super) kind: StableIdKind,
    pub(super) id: Option<String>,
    pub(super) span: SourceSpan,
}

impl StableIdSummary {
    #[must_use]
    pub const fn kind(&self) -> StableIdKind {
        self.kind
    }
    #[must_use]
    pub fn id(&self) -> Option<&str> {
        self.id.as_deref()
    }
    #[must_use]
    pub fn span(&self) -> &SourceSpan {
        &self.span
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum MetadataValueKind {
    Symbol,
    String,
    Integer,
    Float,
    Boolean,
    Array,
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct MetadataSummary {
    pub(super) key: String,
    pub(super) key_span: Option<SourceSpan>,
    pub(super) value_span: Option<SourceSpan>,
    pub(super) value_kind: MetadataValueKind,
}

impl MetadataSummary {
    #[must_use]
    pub fn key(&self) -> &str {
        &self.key
    }
    #[must_use]
    pub fn key_span(&self) -> Option<&SourceSpan> {
        self.key_span.as_ref()
    }
    #[must_use]
    pub fn value_span(&self) -> Option<&SourceSpan> {
        self.value_span.as_ref()
    }
    #[must_use]
    pub const fn value_kind(&self) -> MetadataValueKind {
        self.value_kind
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum FunctionReferenceKind {
    BooleanCondition,
    MatchCondition,
    DeferredEffect,
    ImmediateEffect,
    BlockingEffect,
}

#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub struct FunctionReferenceSummary {
    pub(super) name: String,
    pub(super) span: SourceSpan,
    pub(super) argument_count: usize,
    pub(super) kind: FunctionReferenceKind,
}

impl FunctionReferenceSummary {
    #[must_use]
    pub fn name(&self) -> &str {
        &self.name
    }
    #[must_use]
    pub fn span(&self) -> &SourceSpan {
        &self.span
    }
    #[must_use]
    pub const fn argument_count(&self) -> usize {
        self.argument_count
    }
    #[must_use]
    pub const fn kind(&self) -> FunctionReferenceKind {
        self.kind
    }
}
