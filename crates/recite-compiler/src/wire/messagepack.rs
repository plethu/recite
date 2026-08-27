//! MessagePack v0 row encoders.
//!
//! The `Msg*` tuple structs here are the encode half of the v0 wire format;
//! their field order and arity must match the decoder mirrors in
//! `crates/recite-core/src/compiled/messagepack/wire.rs`, the arity constants
//! in `recite_core::compiled::wire`, and the field tables in
//! `docs/recite-production-spec.md` §12.2. Update all of them together; the
//! tag-surface round-trip and golden wire-byte tests in
//! `recite-compiler/tests/asset/` fail when one drifts.

use recite_core::{
    ChoiceIndex, CompiledDialogue, MatchArmIndex, MetadataIndex, SourceMapIndex, SpeakerIndex,
    StatementIndex, TableRange,
};
use serde::Serialize;
use serde::ser::SerializeTuple;

use self::tags::{
    MsgArgument, MsgAssetEncoding, MsgChoiceEcho, MsgConditionExpression, MsgDivertTarget,
    MsgEffectMode, MsgFingerprint, MsgInspectionEncoding, MsgMatchPattern, MsgSchemaFingerprint,
    MsgSourceSpan, MsgStatementKind, MsgValue,
};
use super::shared::range_to_u32;
use crate::compile::CompileError;

mod tags;

pub(crate) fn serialize_messagepack(dialogue: &CompiledDialogue) -> Result<Vec<u8>, CompileError> {
    rmp_serde::to_vec(&MsgDialogue::from(dialogue)).map_err(|error| {
        CompileError::Serialization(format!("failed to encode MessagePack: {error}"))
    })
}

#[derive(Serialize)]
struct MsgDialogue<'a>(
    MsgHeader<'a>,
    u32,
    Vec<MsgSourceFile<'a>>,
    Vec<MsgBlock<'a>>,
    Vec<MsgStatement<'a>>,
    Vec<MsgMatchArm<'a>>,
    Vec<MsgLine<'a>>,
    Vec<MsgChoice<'a>>,
    Vec<MsgAvailabilityReason<'a>>,
    Vec<MsgConditionAvailabilityReason<'a>>,
    Vec<MsgSpeaker<'a>>,
    Vec<MsgMetadataEntry<'a>>,
    Vec<MsgEffect<'a>>,
    Vec<MsgSourceMapEntry<'a>>,
    Vec<MsgLookupEntry<'a>>,
    Vec<MsgLookupEntry<'a>>,
    Vec<MsgLookupEntry<'a>>,
);

impl<'a> From<&'a CompiledDialogue> for MsgDialogue<'a> {
    fn from(dialogue: &'a CompiledDialogue) -> Self {
        Self(
            MsgHeader::from(dialogue),
            dialogue.default_block.as_u32(),
            dialogue.sources.iter().map(MsgSourceFile::from).collect(),
            dialogue.blocks.iter().map(MsgBlock::from).collect(),
            dialogue.statements.iter().map(MsgStatement::from).collect(),
            dialogue.match_arms.iter().map(MsgMatchArm::from).collect(),
            dialogue.lines.iter().map(MsgLine::from).collect(),
            dialogue.choices.iter().map(MsgChoice::from).collect(),
            dialogue
                .availability_reasons
                .iter()
                .map(MsgAvailabilityReason::from)
                .collect(),
            dialogue
                .condition_availability_reasons
                .iter()
                .map(MsgConditionAvailabilityReason::from)
                .collect(),
            dialogue.speakers.iter().map(MsgSpeaker::from).collect(),
            dialogue
                .metadata
                .iter()
                .map(MsgMetadataEntry::from)
                .collect(),
            dialogue.effects.iter().map(MsgEffect::from).collect(),
            dialogue
                .source_maps
                .iter()
                .map(MsgSourceMapEntry::from)
                .collect(),
            dialogue
                .block_lookup
                .iter()
                .map(|entry| MsgLookupEntry(entry.id.as_str(), entry.index.as_u32()))
                .collect(),
            dialogue
                .line_lookup
                .iter()
                .map(|entry| MsgLookupEntry(entry.id.as_str(), entry.index.as_u32()))
                .collect(),
            dialogue
                .choice_lookup
                .iter()
                .map(|entry| MsgLookupEntry(entry.id.as_str(), entry.index.as_u32()))
                .collect(),
        )
    }
}

#[derive(Serialize)]
struct MsgHeader<'a>(
    u16,
    u16,
    MsgAssetEncoding,
    MsgInspectionEncoding,
    &'a str,
    &'a str,
    &'a str,
    MsgSchemaFingerprint<'a>,
);

impl<'a> From<&'a CompiledDialogue> for MsgHeader<'a> {
    fn from(dialogue: &'a CompiledDialogue) -> Self {
        let header = &dialogue.header;
        Self(
            header.format_version,
            header.compiler_compatibility_version,
            MsgAssetEncoding(header.primary_encoding),
            MsgInspectionEncoding(header.inspection_encoding),
            header.compiler_version.as_str(),
            header.asset_id.as_str(),
            header.source_map_id.as_str(),
            MsgSchemaFingerprint(&header.schema_fingerprint),
        )
    }
}

#[derive(Serialize)]
struct MsgSourceFile<'a>(&'a str, MsgFingerprint<'a>);

impl<'a> From<&'a recite_core::CompiledSourceFile> for MsgSourceFile<'a> {
    fn from(source: &'a recite_core::CompiledSourceFile) -> Self {
        Self(source.path.as_str(), MsgFingerprint(&source.fingerprint))
    }
}

#[derive(Serialize)]
struct MsgBlock<'a>(&'a str, u32, MsgRange, MsgRange, Option<u32>, u32);

impl<'a> From<&'a recite_core::CompiledBlock> for MsgBlock<'a> {
    fn from(block: &'a recite_core::CompiledBlock) -> Self {
        Self(
            block.id.as_str(),
            block.source_file.as_u32(),
            statement_range(block.statements),
            metadata_range(block.metadata),
            block.default_speaker.map(SpeakerIndex::as_u32),
            block.source_map.as_u32(),
        )
    }
}

#[derive(Serialize)]
struct MsgStatement<'a>(MsgStatementKind<'a>, u32);

impl<'a> From<&'a recite_core::CompiledStatement> for MsgStatement<'a> {
    fn from(statement: &'a recite_core::CompiledStatement) -> Self {
        Self(
            MsgStatementKind(&statement.kind),
            statement.source_map.as_u32(),
        )
    }
}

#[derive(Serialize)]
struct MsgMatchArm<'a>(MsgMatchPattern<'a>, MsgRange, u32);

impl<'a> From<&'a recite_core::CompiledMatchArm> for MsgMatchArm<'a> {
    fn from(arm: &'a recite_core::CompiledMatchArm) -> Self {
        Self(
            MsgMatchPattern(&arm.pattern),
            statement_range(arm.statements),
            arm.source_map.as_u32(),
        )
    }
}

#[derive(Serialize)]
struct MsgLine<'a>(&'a str, &'a str, Option<u32>, MsgRange, u32);

impl<'a> From<&'a recite_core::CompiledLine> for MsgLine<'a> {
    fn from(line: &'a recite_core::CompiledLine) -> Self {
        Self(
            line.id.as_str(),
            line.source_text.as_str(),
            line.speaker.map(SpeakerIndex::as_u32),
            metadata_range(line.metadata),
            line.source_map.as_u32(),
        )
    }
}

#[derive(Serialize)]
struct MsgChoice<'a>(
    &'a str,
    &'a str,
    MsgRange,
    Option<MsgConditionExpression<'a>>,
    Option<&'a str>,
    Option<&'a str>,
    MsgDivertTarget<'a>,
    MsgChoiceEcho<'a>,
    u32,
);

impl<'a> From<&'a recite_core::CompiledChoice> for MsgChoice<'a> {
    fn from(choice: &'a recite_core::CompiledChoice) -> Self {
        Self(
            choice.id.as_str(),
            choice.source_text.as_str(),
            metadata_range(choice.metadata),
            choice
                .availability_requirement
                .as_ref()
                .map(MsgConditionExpression),
            choice.availability_requirement_source_text.as_deref(),
            choice
                .availability_reason_override
                .as_ref()
                .map(recite_core::AvailabilityReasonId::as_str),
            MsgDivertTarget(&choice.target),
            MsgChoiceEcho(&choice.echo),
            choice.source_map.as_u32(),
        )
    }
}

#[derive(Serialize)]
struct MsgAvailabilityReason<'a>(&'a str, &'a str);

impl<'a> From<&'a recite_core::CompiledAvailabilityReason> for MsgAvailabilityReason<'a> {
    fn from(reason: &'a recite_core::CompiledAvailabilityReason) -> Self {
        Self(reason.id.as_str(), reason.template.as_str())
    }
}

#[derive(Serialize)]
struct MsgConditionAvailabilityReason<'a>(
    &'a str,
    &'a str,
    Vec<MsgAvailabilityReasonArgBinding<'a>>,
);

impl<'a> From<&'a recite_core::CompiledConditionAvailabilityReason>
    for MsgConditionAvailabilityReason<'a>
{
    fn from(mapping: &'a recite_core::CompiledConditionAvailabilityReason) -> Self {
        Self(
            mapping.function.as_str(),
            mapping.reason.as_str(),
            mapping
                .args
                .iter()
                .map(MsgAvailabilityReasonArgBinding::from)
                .collect(),
        )
    }
}

#[derive(Serialize)]
struct MsgAvailabilityReasonArgBinding<'a>(&'a str, MsgAvailabilityReasonArgValue<'a>);

impl<'a> From<&'a recite_core::CompiledAvailabilityReasonArgBinding>
    for MsgAvailabilityReasonArgBinding<'a>
{
    fn from(binding: &'a recite_core::CompiledAvailabilityReasonArgBinding) -> Self {
        Self(
            binding.name.as_str(),
            MsgAvailabilityReasonArgValue(&binding.value),
        )
    }
}

struct MsgAvailabilityReasonArgValue<'a>(&'a recite_core::CompiledAvailabilityReasonArgValue);

impl Serialize for MsgAvailabilityReasonArgValue<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(recite_core::V0_TAGGED_VALUE_FIELDS as usize)?;
        match self.0 {
            recite_core::CompiledAvailabilityReasonArgValue::ConditionArg(value) => {
                tuple.serialize_element("ConditionArg")?;
                tuple.serialize_element(value)?;
            }
            recite_core::CompiledAvailabilityReasonArgValue::Literal(value) => match value {
                recite_core::ScalarValue::String(value) => {
                    tuple.serialize_element("LiteralString")?;
                    tuple.serialize_element(value)?;
                }
                recite_core::ScalarValue::Integer(value) => {
                    tuple.serialize_element("LiteralInt")?;
                    tuple.serialize_element(value)?;
                }
                recite_core::ScalarValue::Float(value) => {
                    tuple.serialize_element("LiteralFloat")?;
                    tuple.serialize_element(value)?;
                }
                recite_core::ScalarValue::Boolean(value) => {
                    tuple.serialize_element("LiteralBool")?;
                    tuple.serialize_element(value)?;
                }
            },
        }
        tuple.end()
    }
}

struct MsgSpeaker<'a>(&'a str);

impl<'a> From<&'a recite_core::CompiledSpeaker> for MsgSpeaker<'a> {
    fn from(speaker: &'a recite_core::CompiledSpeaker) -> Self {
        Self(speaker.id.as_str())
    }
}

impl Serialize for MsgSpeaker<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(recite_core::V0_SPEAKER_FIELDS as usize)?;
        tuple.serialize_element(&self.0)?;
        tuple.end()
    }
}

#[derive(Serialize)]
struct MsgMetadataEntry<'a>(&'a str, MsgValue<'a>, Option<u32>);

impl<'a> From<&'a recite_core::CompiledMetadataEntry> for MsgMetadataEntry<'a> {
    fn from(entry: &'a recite_core::CompiledMetadataEntry) -> Self {
        Self(
            entry.key.as_str(),
            MsgValue(&entry.value),
            entry.source_map.map(SourceMapIndex::as_u32),
        )
    }
}

#[derive(Serialize)]
struct MsgEffect<'a>(&'a str, MsgEffectMode, &'a str, Vec<MsgArgument<'a>>, u32);

impl<'a> From<&'a recite_core::CompiledEffect> for MsgEffect<'a> {
    fn from(effect: &'a recite_core::CompiledEffect) -> Self {
        Self(
            effect.id.as_str(),
            MsgEffectMode(effect.mode),
            effect.function.as_str(),
            effect.args.iter().map(MsgArgument).collect(),
            effect.source_map.as_u32(),
        )
    }
}

#[derive(Serialize)]
struct MsgSourceMapEntry<'a>(u32, MsgSourceSpan<'a>);

impl<'a> From<&'a recite_core::CompiledSourceMapEntry> for MsgSourceMapEntry<'a> {
    fn from(entry: &'a recite_core::CompiledSourceMapEntry) -> Self {
        Self(entry.source_file.as_u32(), MsgSourceSpan(&entry.span))
    }
}

#[derive(Serialize)]
struct MsgLookupEntry<'a>(&'a str, u32);

#[derive(Serialize)]
struct MsgRange(u32, u32);

fn statement_range(range: TableRange<StatementIndex>) -> MsgRange {
    let (start, len) = range_to_u32(range, StatementIndex::as_u32);
    MsgRange(start, len)
}

fn match_arm_range(range: TableRange<MatchArmIndex>) -> MsgRange {
    let (start, len) = range_to_u32(range, MatchArmIndex::as_u32);
    MsgRange(start, len)
}

fn choice_range(range: TableRange<ChoiceIndex>) -> MsgRange {
    let (start, len) = range_to_u32(range, ChoiceIndex::as_u32);
    MsgRange(start, len)
}

fn metadata_range(range: TableRange<MetadataIndex>) -> MsgRange {
    let (start, len) = range_to_u32(range, MetadataIndex::as_u32);
    MsgRange(start, len)
}
