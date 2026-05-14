use crate::{BlockId, ChoiceId, EffectId, LineId, ScalarValue, SourceSpan, SpeakerId, Value};

use super::{
    BlockIndex, BlockLookupTable, ChoiceLookupTable, ChoiceRange, CompiledAssetHeader,
    ContentFingerprint, EffectIndex, LineIndex, LineLookupTable, MetadataRange, SourceFileIndex,
    SourceMapIndex, SpeakerIndex, StatementRange,
};

/// Runtime-facing compiled dialogue asset.
#[derive(Clone, Debug, PartialEq)]
pub struct CompiledDialogue {
    pub header: CompiledAssetHeader,
    pub sources: Vec<CompiledSourceFile>,
    pub blocks: Vec<CompiledBlock>,
    pub statements: Vec<CompiledStatement>,
    pub lines: Vec<CompiledLine>,
    pub choices: Vec<CompiledChoice>,
    pub speakers: Vec<CompiledSpeaker>,
    pub metadata: Vec<CompiledMetadataEntry>,
    pub effects: Vec<CompiledEffect>,
    pub source_maps: Vec<CompiledSourceMapEntry>,
    pub block_lookup: BlockLookupTable,
    pub line_lookup: LineLookupTable,
    pub choice_lookup: ChoiceLookupTable,
}

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
    pub condition: Option<CompiledConditionExpression>,
    pub target: CompiledDivertTarget,
    pub echo: CompiledChoiceEcho,
    pub source_map: SourceMapIndex,
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
