#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct FileSummaryCompleteness {
    pub(crate) block_definitions: bool,
    pub(crate) metadata: bool,
    pub(crate) condition_functions: bool,
    pub(crate) effect_functions: bool,
    pub(crate) inline_markup: bool,
    pub(crate) recoverable_regions: bool,
}

#[cfg(any(test, feature = "bench-support"))]
#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct SpannedName {
    pub(crate) name: String,
    pub(crate) span: SourceSpan,
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
use recite_core::SourceSpan;
