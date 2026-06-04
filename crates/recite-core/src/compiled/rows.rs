use crate::{
    AvailabilityReasonId, BlockId, ChoiceId, EffectId, LineId, ScalarValue, SourceSpan, SpeakerId,
    Value,
};

use super::{
    BlockIndex, BlockLookupTable, ChoiceLookupTable, ChoiceRange, CompiledAssetHeader,
    ContentFingerprint, EffectIndex, LineIndex, LineLookupTable, MatchArmRange, MetadataRange,
    SourceFileIndex, SourceMapIndex, SpeakerIndex, StatementRange,
};

/// Runtime-facing compiled dialogue asset.
#[derive(Clone, Debug, PartialEq)]
pub struct CompiledDialogue {
    pub header: CompiledAssetHeader,
    pub default_block: BlockIndex,
    pub sources: Vec<CompiledSourceFile>,
    pub blocks: Vec<CompiledBlock>,
    pub statements: Vec<CompiledStatement>,
    pub match_arms: Vec<CompiledMatchArm>,
    pub lines: Vec<CompiledLine>,
    pub choices: Vec<CompiledChoice>,
    pub availability_reasons: Vec<CompiledAvailabilityReason>,
    pub condition_availability_reasons: Vec<CompiledConditionAvailabilityReason>,
    pub speakers: Vec<CompiledSpeaker>,
    pub metadata: Vec<CompiledMetadataEntry>,
    pub effects: Vec<CompiledEffect>,
    pub source_maps: Vec<CompiledSourceMapEntry>,
    pub block_lookup: BlockLookupTable,
    pub line_lookup: LineLookupTable,
    pub choice_lookup: ChoiceLookupTable,
}

/// A compiled asset is a read-only program the runtime borrows per call, so it
/// must stay shareable across threads (e.g. wrapped in `Arc` behind a worker
/// pool). This guard fails to compile if a future field reintroduces a
/// thread-unsafe type such as `Rc`, `Cell`, or `RefCell`.
const _: fn() = || {
    fn assert_send_sync<T: Send + Sync + 'static>() {}
    assert_send_sync::<CompiledDialogue>();
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledSourceFile {
    pub path: String,
    pub fingerprint: ContentFingerprint,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledBlock {
    pub id: BlockId,
    pub source_file: SourceFileIndex,
    pub statements: StatementRange,
    pub metadata: MetadataRange,
    pub default_speaker: Option<SpeakerIndex>,
    pub source_map: SourceMapIndex,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompiledStatement {
    pub kind: CompiledStatementKind,
    pub source_map: SourceMapIndex,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CompiledStatementKind {
    Line(LineIndex),
    Prompt {
        line: Option<LineIndex>,
        choices: ChoiceRange,
    },
    Divert(CompiledDivertTarget),
    If {
        condition: CompiledConditionExpression,
        then_statements: StatementRange,
        else_statements: StatementRange,
    },
    Match {
        scrutinee: CompiledConditionCall,
        arms: MatchArmRange,
    },
    Effect(EffectIndex),
    End,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompiledDivertTarget {
    Block(BlockIndex),
    End,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompiledLine {
    pub id: LineId,
    pub source_text: String,
    pub speaker: Option<SpeakerIndex>,
    pub metadata: MetadataRange,
    pub source_map: SourceMapIndex,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompiledChoice {
    pub id: ChoiceId,
    pub source_text: String,
    pub metadata: MetadataRange,
    pub availability_requirement: Option<CompiledConditionExpression>,
    pub availability_requirement_source_text: Option<String>,
    pub availability_reason_override: Option<AvailabilityReasonId>,
    pub target: CompiledDivertTarget,
    pub echo: CompiledChoiceEcho,
    pub source_map: SourceMapIndex,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledAvailabilityReason {
    pub id: AvailabilityReasonId,
    pub template: String,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompiledConditionAvailabilityReason {
    pub function: String,
    pub reason: AvailabilityReasonId,
    pub args: Vec<CompiledAvailabilityReasonArgBinding>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompiledAvailabilityReasonArgBinding {
    pub name: String,
    pub value: CompiledAvailabilityReasonArgValue,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CompiledAvailabilityReasonArgValue {
    ConditionArg(String),
    Literal(String),
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompiledMatchArm {
    pub pattern: CompiledMatchPattern,
    pub statements: StatementRange,
    pub source_map: SourceMapIndex,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompiledMatchPattern {
    Variant(String),
    Wildcard,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompiledChoiceEcho {
    None,
    SelectedText,
    ExplicitLine(LineId),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledSpeaker {
    pub id: SpeakerId,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompiledMetadataEntry {
    pub key: String,
    pub value: Value,
    pub source_map: Option<SourceMapIndex>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompiledEffect {
    pub id: EffectId,
    pub mode: CompiledEffectMode,
    pub function: String,
    pub args: Vec<CompiledArgument>,
    pub source_map: SourceMapIndex,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Hash)]
pub enum CompiledEffectMode {
    Deferred,
    Immediate,
    Blocking,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CompiledConditionExpression {
    Call(CompiledConditionCall),
    And(Vec<CompiledConditionExpression>),
    Or(Vec<CompiledConditionExpression>),
    Not(Box<CompiledConditionExpression>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct CompiledConditionCall {
    pub function: String,
    pub args: Vec<CompiledArgument>,
}

#[derive(Clone, Debug, PartialEq)]
pub enum CompiledArgument {
    Identifier(String),
    Value(ScalarValue),
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CompiledSourceMapEntry {
    pub source_file: SourceFileIndex,
    pub span: SourceSpan,
}
