#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct FileSummaryCompleteness {
    pub(crate) block_definitions: bool,
    pub(crate) block_references: bool,
    pub(crate) stable_ids: bool,
    pub(crate) metadata: bool,
    pub(crate) condition_functions: bool,
    pub(crate) effect_functions: bool,
    pub(crate) inline_markup: bool,
    pub(crate) recoverable_regions: bool,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SpannedName {
    pub(crate) name: String,
    pub(crate) span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct BlockReferenceSummary {
    pub(crate) file: Option<String>,
    pub(crate) block_id: String,
    pub(crate) span: SourceSpan,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MissingIdSummary {
    pub(crate) kind: MissingIdKind,
    pub(crate) label: Option<String>,
    pub(crate) insertion: MissingIdInsertion,
    pub(crate) span: SourceSpan,
    pub(crate) insertion_position: SourcePosition,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum MissingIdInsertion {
    FullId,
    AnchorOnly,
    AtAnchor,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub(crate) enum MissingIdKind {
    Line,
    Choice,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct MetadataKeySummary {
    pub(crate) key: String,
    pub(crate) key_span: Option<SourceSpan>,
    pub(crate) entry_span: Option<SourceSpan>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct FunctionReferenceSummary {
    pub(crate) name: String,
    pub(crate) span: SourceSpan,
    pub(crate) argument_count: usize,
    pub(crate) kind: FunctionReferenceKind,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum FunctionReferenceKind {
    BoolCondition,
    MatchCondition,
    DeferredEffect,
    ImmediateEffect,
    BlockingEffect,
}
use recite_core::{SourcePosition, SourceSpan};
