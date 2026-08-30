//! MessagePack v0 row decoders.
//!
//! The `Msg*` types here are the decode half of the v0 wire format; their
//! field order and arity must match the canonical encoder in
//! `crate::compiled::messagepack::encode`, the arity constants in
//! `crate::compiled::wire`, and the field tables in
//! `docs/recite-production-spec.md` §12.2. Update all of them together; the
//! tag-surface round-trip and golden wire-byte tests in
//! `recite-compiler/tests/asset/` fail when one drifts.

use serde::Deserialize;
use serde::de::{IntoDeserializer, SeqAccess, Visitor};

use crate::{AvailabilityReasonId, BlockId, ChoiceId, EffectId, LineId, ScalarValue, SpeakerId};

use super::CompiledAssetDecodeError;
use super::interpolation::MsgInterpolationBinding;
use super::tags::{
    MsgArgument, MsgAssetEncoding, MsgChoiceEcho, MsgConditionExpression, MsgDivertTarget,
    MsgEffectMode, MsgFingerprint, MsgInspectionEncoding, MsgMatchPattern, MsgSchemaFingerprint,
    MsgSourceSpan, MsgStatementKind, MsgValue, collect_wrapped,
};
use crate::compiled::{
    BlockIndex, BlockLookupEntry, BlockLookupTable, COMPILED_ASSET_FORMAT_VERSION_V0,
    COMPILER_COMPATIBILITY_VERSION_V0, ChoiceIndex, ChoiceLookupEntry, ChoiceLookupTable,
    ChoiceRange, CompiledAssetHeader, CompiledAssetId, CompiledBlock, CompiledChoice,
    CompiledDialogue, CompiledEffect, CompiledInterpolationMode, CompiledLine, CompiledMatchArm,
    CompiledMetadataEntry, CompiledSourceFile, CompiledSourceMapEntry, CompiledSpeaker,
    CompiledStatement, CompilerVersion, LineIndex, LineLookupEntry, LineLookupTable, MatchArmIndex,
    MatchArmRange, MetadataIndex, MetadataRange, SourceFileIndex, SourceMapId, SourceMapIndex,
    SpeakerIndex, StatementIndex, StatementRange, TableRange, V0_TAGGED_VALUE_FIELDS,
};

#[derive(Deserialize)]
pub(super) struct MsgDialogue(
    MsgHeader,
    u32,
    Vec<MsgSourceFile>,
    Vec<MsgBlock>,
    Vec<MsgStatement>,
    Vec<MsgMatchArm>,
    Vec<MsgLine>,
    Vec<MsgChoice>,
    Vec<MsgAvailabilityReason>,
    Vec<MsgConditionAvailabilityReason>,
    Vec<MsgSpeaker>,
    Vec<MsgMetadataEntry>,
    Vec<MsgEffect>,
    Vec<MsgSourceMapEntry>,
    Vec<MsgLookupEntry>,
    Vec<MsgLookupEntry>,
    Vec<MsgLookupEntry>,
);

impl TryFrom<MsgDialogue> for CompiledDialogue {
    type Error = CompiledAssetDecodeError;

    fn try_from(value: MsgDialogue) -> Result<Self, Self::Error> {
        Ok(Self {
            header: value.0.try_into()?,
            default_block: BlockIndex::new(value.1),
            sources: collect(value.2)?,
            blocks: collect(value.3)?,
            statements: collect(value.4)?,
            match_arms: collect(value.5)?,
            lines: collect(value.6)?,
            choices: collect(value.7)?,
            availability_reasons: collect(value.8)?,
            condition_availability_reasons: collect(value.9)?,
            speakers: collect(value.10)?,
            metadata: collect(value.11)?,
            effects: collect(value.12)?,
            source_maps: collect(value.13)?,
            block_lookup: BlockLookupTable::new(
                value
                    .14
                    .into_iter()
                    .map(|entry| entry.block())
                    .collect::<Result<Vec<_>, _>>()?,
            )?,
            line_lookup: LineLookupTable::new(
                value
                    .15
                    .into_iter()
                    .map(|entry| entry.line())
                    .collect::<Result<Vec<_>, _>>()?,
            )?,
            choice_lookup: ChoiceLookupTable::new(
                value
                    .16
                    .into_iter()
                    .map(|entry| entry.choice())
                    .collect::<Result<Vec<_>, _>>()?,
            )?,
        })
    }
}

#[derive(Deserialize)]
struct MsgHeader(
    u16,
    u16,
    MsgAssetEncoding,
    MsgInspectionEncoding,
    String,
    String,
    String,
    MsgSchemaFingerprint,
);

impl TryFrom<MsgHeader> for CompiledAssetHeader {
    type Error = CompiledAssetDecodeError;

    fn try_from(value: MsgHeader) -> Result<Self, Self::Error> {
        if value.0 != COMPILED_ASSET_FORMAT_VERSION_V0
            || value.1 != COMPILER_COMPATIBILITY_VERSION_V0
        {
            return Err(CompiledAssetDecodeError::UnsupportedFormat {
                format_version: value.0,
                compiler_compatibility_version: value.1,
            });
        }

        Ok(Self {
            format_version: value.0,
            compiler_compatibility_version: value.1,
            primary_encoding: value.2.0,
            inspection_encoding: value.3.0,
            compiler_version: CompilerVersion::new(value.4)?,
            asset_id: CompiledAssetId::new(value.5)?,
            source_map_id: SourceMapId::new(value.6)?,
            schema_fingerprint: value.7.0,
        })
    }
}

#[derive(Deserialize)]
struct MsgSourceFile(String, MsgFingerprint);

impl TryFrom<MsgSourceFile> for CompiledSourceFile {
    type Error = CompiledAssetDecodeError;

    fn try_from(value: MsgSourceFile) -> Result<Self, Self::Error> {
        Ok(Self {
            path: value.0,
            fingerprint: value.1.0,
        })
    }
}

#[derive(Deserialize)]
struct MsgBlock(String, u32, MsgRange, MsgRange, Option<u32>, u32);

impl TryFrom<MsgBlock> for CompiledBlock {
    type Error = CompiledAssetDecodeError;

    fn try_from(value: MsgBlock) -> Result<Self, Self::Error> {
        Ok(Self {
            id: BlockId::new(value.0)?,
            source_file: SourceFileIndex::new(value.1),
            statements: value.2.statement(),
            metadata: value.3.metadata(),
            default_speaker: value.4.map(SpeakerIndex::new),
            source_map: SourceMapIndex::new(value.5),
        })
    }
}

#[derive(Deserialize)]
struct MsgStatement(MsgStatementKind, u32);

impl TryFrom<MsgStatement> for CompiledStatement {
    type Error = CompiledAssetDecodeError;

    fn try_from(value: MsgStatement) -> Result<Self, Self::Error> {
        Ok(Self {
            kind: value.0.0,
            source_map: SourceMapIndex::new(value.1),
        })
    }
}

#[derive(Deserialize)]
struct MsgMatchArm(MsgMatchPattern, MsgRange, u32);

impl TryFrom<MsgMatchArm> for CompiledMatchArm {
    type Error = CompiledAssetDecodeError;

    fn try_from(value: MsgMatchArm) -> Result<Self, Self::Error> {
        Ok(Self {
            pattern: value.0.0,
            statements: value.1.statement(),
            source_map: SourceMapIndex::new(value.2),
        })
    }
}

enum MsgLine {
    Plural(MsgLinePlural),
    Current(MsgLineCurrent),
    Legacy(MsgLineLegacy),
}

impl<'de> Deserialize<'de> for MsgLine {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_value::Value::deserialize(deserializer)?;
        let len = match &value {
            serde_value::Value::Seq(values) => values.len(),
            _ => 0,
        };
        match len {
            9 => MsgLinePlural::deserialize(value.into_deserializer())
                .map(Self::Plural)
                .map_err(serde::de::Error::custom),
            7 => MsgLineCurrent::deserialize(value.into_deserializer())
                .map(Self::Current)
                .map_err(serde::de::Error::custom),
            5 => MsgLineLegacy::deserialize(value.into_deserializer())
                .map(Self::Legacy)
                .map_err(serde::de::Error::custom),
            _ => Err(serde::de::Error::custom(
                "compiled line must have 5, 7, or 9 fields",
            )),
        }
    }
}

#[derive(Deserialize)]
struct MsgLineCurrent(
    String,
    String,
    Option<u32>,
    MsgRange,
    u32,
    String,
    Vec<MsgInterpolationBinding>,
);

#[derive(Deserialize)]
struct MsgLinePlural(
    String,
    String,
    Option<u32>,
    MsgRange,
    u32,
    String,
    Vec<MsgInterpolationBinding>,
    Option<String>,
    Option<String>,
);

#[derive(Deserialize)]
struct MsgLineLegacy(String, String, Option<u32>, MsgRange, u32);

impl TryFrom<MsgLine> for CompiledLine {
    type Error = CompiledAssetDecodeError;

    fn try_from(value: MsgLine) -> Result<Self, Self::Error> {
        match value {
            MsgLine::Plural(value) => {
                let line = Self {
                    id: LineId::new(value.0)?,
                    source_text: value.1,
                    plural_source_text: value.7,
                    speaker: value.2.map(SpeakerIndex::new),
                    metadata: value.3.metadata(),
                    source_map: SourceMapIndex::new(value.4),
                    authored_source_text: value.5,
                    authored_plural_source_text: value.8,
                    interpolation_bindings: value
                        .6
                        .into_iter()
                        .map(TryInto::try_into)
                        .collect::<Result<_, _>>()?,
                    interpolation_mode: CompiledInterpolationMode::Current,
                };
                Ok(line)
            }
            MsgLine::Current(value) => {
                let line = Self {
                    id: LineId::new(value.0)?,
                    source_text: value.1,
                    plural_source_text: None,
                    speaker: value.2.map(SpeakerIndex::new),
                    metadata: value.3.metadata(),
                    source_map: SourceMapIndex::new(value.4),
                    authored_source_text: value.5,
                    interpolation_bindings: value
                        .6
                        .into_iter()
                        .map(TryInto::try_into)
                        .collect::<Result<_, _>>()?,
                    interpolation_mode: CompiledInterpolationMode::Current,
                    authored_plural_source_text: None,
                };
                Ok(line)
            }
            MsgLine::Legacy(value) => Ok(Self {
                id: LineId::new(value.0)?,
                source_text: value.1.clone(),
                plural_source_text: None,
                authored_source_text: value.1,
                authored_plural_source_text: None,
                interpolation_bindings: Vec::new(),
                interpolation_mode: CompiledInterpolationMode::Legacy,
                speaker: value.2.map(SpeakerIndex::new),
                metadata: value.3.metadata(),
                source_map: SourceMapIndex::new(value.4),
            }),
        }
    }
}

enum MsgChoice {
    Current(MsgChoiceCurrent),
    Legacy(MsgChoiceLegacy),
}

impl<'de> Deserialize<'de> for MsgChoice {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let value = serde_value::Value::deserialize(deserializer)?;
        let len = match &value {
            serde_value::Value::Seq(values) => values.len(),
            _ => 0,
        };
        match len {
            11 => MsgChoiceCurrent::deserialize(value.into_deserializer())
                .map(Self::Current)
                .map_err(serde::de::Error::custom),
            9 => MsgChoiceLegacy::deserialize(value.into_deserializer())
                .map(Self::Legacy)
                .map_err(serde::de::Error::custom),
            _ => Err(serde::de::Error::custom(
                "compiled choice must have 11 fields",
            )),
        }
    }
}

#[derive(Deserialize)]
struct MsgChoiceCurrent(
    String,
    String,
    MsgRange,
    Option<MsgConditionExpression>,
    Option<String>,
    Option<String>,
    MsgDivertTarget,
    MsgChoiceEcho,
    u32,
    String,
    Vec<MsgInterpolationBinding>,
);

#[derive(Deserialize)]
struct MsgChoiceLegacy(
    String,
    String,
    MsgRange,
    Option<MsgConditionExpression>,
    Option<String>,
    Option<String>,
    MsgDivertTarget,
    MsgChoiceEcho,
    u32,
);

impl TryFrom<MsgChoice> for CompiledChoice {
    type Error = CompiledAssetDecodeError;

    fn try_from(value: MsgChoice) -> Result<Self, Self::Error> {
        match value {
            MsgChoice::Current(value) => {
                let choice = Self {
                    id: ChoiceId::new(value.0)?,
                    source_text: value.1,
                    metadata: value.2.metadata(),
                    availability_requirement: value.3.map(|condition| condition.0),
                    availability_requirement_source_text: value.4,
                    availability_reason_override: value
                        .5
                        .map(AvailabilityReasonId::new)
                        .transpose()?,
                    target: value.6.0,
                    echo: value.7.0,
                    source_map: SourceMapIndex::new(value.8),
                    authored_source_text: value.9,
                    interpolation_bindings: value
                        .10
                        .into_iter()
                        .map(TryInto::try_into)
                        .collect::<Result<_, _>>()?,
                    interpolation_mode: CompiledInterpolationMode::Current,
                };
                Ok(choice)
            }
            MsgChoice::Legacy(value) => Ok(Self {
                id: ChoiceId::new(value.0)?,
                source_text: value.1.clone(),
                authored_source_text: value.1,
                interpolation_bindings: Vec::new(),
                interpolation_mode: CompiledInterpolationMode::Legacy,
                metadata: value.2.metadata(),
                availability_requirement: value.3.map(|condition| condition.0),
                availability_requirement_source_text: value.4,
                availability_reason_override: value.5.map(AvailabilityReasonId::new).transpose()?,
                target: value.6.0,
                echo: value.7.0,
                source_map: SourceMapIndex::new(value.8),
            }),
        }
    }
}

#[derive(Deserialize)]
struct MsgAvailabilityReason(String, String);

impl TryFrom<MsgAvailabilityReason> for crate::compiled::CompiledAvailabilityReason {
    type Error = CompiledAssetDecodeError;

    fn try_from(value: MsgAvailabilityReason) -> Result<Self, Self::Error> {
        Ok(Self {
            id: AvailabilityReasonId::new(value.0)?,
            template: value.1,
        })
    }
}

#[derive(Deserialize)]
struct MsgConditionAvailabilityReason(String, String, Vec<MsgAvailabilityReasonArgBinding>);

impl TryFrom<MsgConditionAvailabilityReason>
    for crate::compiled::CompiledConditionAvailabilityReason
{
    type Error = CompiledAssetDecodeError;

    fn try_from(value: MsgConditionAvailabilityReason) -> Result<Self, Self::Error> {
        Ok(Self {
            function: value.0,
            reason: AvailabilityReasonId::new(value.1)?,
            args: collect(value.2)?,
        })
    }
}

#[derive(Deserialize)]
struct MsgAvailabilityReasonArgBinding(String, MsgAvailabilityReasonArgValueWrapper);

impl TryFrom<MsgAvailabilityReasonArgBinding>
    for crate::compiled::CompiledAvailabilityReasonArgBinding
{
    type Error = CompiledAssetDecodeError;

    fn try_from(value: MsgAvailabilityReasonArgBinding) -> Result<Self, Self::Error> {
        Ok(Self {
            name: value.0,
            value: value.1.0,
        })
    }
}

struct MsgAvailabilityReasonArgValueWrapper(crate::compiled::CompiledAvailabilityReasonArgValue);

impl<'de> Deserialize<'de> for MsgAvailabilityReasonArgValueWrapper {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_tuple(
            V0_TAGGED_VALUE_FIELDS as usize,
            MsgAvailabilityReasonArgValueVisitor,
        )
    }
}

struct MsgAvailabilityReasonArgValueVisitor;

impl<'de> Visitor<'de> for MsgAvailabilityReasonArgValueVisitor {
    type Value = MsgAvailabilityReasonArgValueWrapper;

    fn expecting(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str("availability reason argument value tuple")
    }

    fn visit_seq<A>(self, mut seq: A) -> Result<Self::Value, A::Error>
    where
        A: SeqAccess<'de>,
    {
        let tag = seq
            .next_element::<String>()?
            .ok_or_else(|| serde::de::Error::invalid_length(0, &self))?;
        match tag.as_str() {
            "ConditionArg" => {
                let value = seq
                    .next_element::<u32>()?
                    .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;
                Ok(MsgAvailabilityReasonArgValueWrapper(
                    crate::compiled::CompiledAvailabilityReasonArgValue::ConditionArg(value),
                ))
            }
            "LiteralString" => Ok(MsgAvailabilityReasonArgValueWrapper(
                crate::compiled::CompiledAvailabilityReasonArgValue::Literal(ScalarValue::String(
                    seq.next_element::<String>()?
                        .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?,
                )),
            )),
            "LiteralInt" => Ok(MsgAvailabilityReasonArgValueWrapper(
                crate::compiled::CompiledAvailabilityReasonArgValue::Literal(ScalarValue::Integer(
                    seq.next_element::<i64>()?
                        .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?,
                )),
            )),
            "LiteralFloat" => {
                let value = seq
                    .next_element::<f64>()?
                    .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?;
                Ok(MsgAvailabilityReasonArgValueWrapper(
                    crate::compiled::CompiledAvailabilityReasonArgValue::Literal(
                        ScalarValue::Float(value),
                    ),
                ))
            }
            "LiteralBool" => Ok(MsgAvailabilityReasonArgValueWrapper(
                crate::compiled::CompiledAvailabilityReasonArgValue::Literal(ScalarValue::Boolean(
                    seq.next_element::<bool>()?
                        .ok_or_else(|| serde::de::Error::invalid_length(1, &self))?,
                )),
            )),
            _ => Err(serde::de::Error::custom(format!(
                "unknown availability reason argument value tag `{tag}`"
            ))),
        }
    }
}

struct MsgSpeaker(String);

impl<'de> Deserialize<'de> for MsgSpeaker {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (id,): (String,) = Deserialize::deserialize(deserializer)?;
        Ok(Self(id))
    }
}

impl TryFrom<MsgSpeaker> for CompiledSpeaker {
    type Error = CompiledAssetDecodeError;

    fn try_from(value: MsgSpeaker) -> Result<Self, Self::Error> {
        Ok(Self {
            id: SpeakerId::new(value.0)?,
        })
    }
}

#[derive(Deserialize)]
struct MsgMetadataEntry(String, MsgValue, Option<u32>);

impl TryFrom<MsgMetadataEntry> for CompiledMetadataEntry {
    type Error = CompiledAssetDecodeError;

    fn try_from(value: MsgMetadataEntry) -> Result<Self, Self::Error> {
        Ok(Self {
            key: value.0,
            value: value.1.0,
            source_map: value.2.map(SourceMapIndex::new),
        })
    }
}

#[derive(Deserialize)]
struct MsgEffect(String, MsgEffectMode, String, Vec<MsgArgument>, u32);

impl TryFrom<MsgEffect> for CompiledEffect {
    type Error = CompiledAssetDecodeError;

    fn try_from(value: MsgEffect) -> Result<Self, Self::Error> {
        Ok(Self {
            id: EffectId::new(value.0)?,
            mode: value.1.0,
            function: value.2,
            args: collect_wrapped(value.3),
            source_map: SourceMapIndex::new(value.4),
        })
    }
}

#[derive(Deserialize)]
struct MsgSourceMapEntry(u32, MsgSourceSpan);

impl TryFrom<MsgSourceMapEntry> for CompiledSourceMapEntry {
    type Error = CompiledAssetDecodeError;

    fn try_from(value: MsgSourceMapEntry) -> Result<Self, Self::Error> {
        Ok(Self {
            source_file: SourceFileIndex::new(value.0),
            span: value.1.0,
        })
    }
}

#[derive(Deserialize)]
struct MsgLookupEntry(String, u32);

impl MsgLookupEntry {
    fn block(self) -> Result<BlockLookupEntry, CompiledAssetDecodeError> {
        Ok(BlockLookupEntry {
            id: BlockId::new(self.0)?,
            index: BlockIndex::new(self.1),
        })
    }

    fn line(self) -> Result<LineLookupEntry, CompiledAssetDecodeError> {
        Ok(LineLookupEntry {
            id: LineId::new(self.0)?,
            index: LineIndex::new(self.1),
        })
    }

    fn choice(self) -> Result<ChoiceLookupEntry, CompiledAssetDecodeError> {
        Ok(ChoiceLookupEntry {
            id: ChoiceId::new(self.0)?,
            index: ChoiceIndex::new(self.1),
        })
    }
}

#[derive(Deserialize)]
pub(super) struct MsgRange(u32, u32);

impl MsgRange {
    pub(super) fn statement(self) -> StatementRange {
        TableRange::new(StatementIndex::new(self.0), self.1)
    }

    pub(super) fn match_arm(self) -> MatchArmRange {
        TableRange::new(MatchArmIndex::new(self.0), self.1)
    }

    pub(super) fn choice(self) -> ChoiceRange {
        TableRange::new(ChoiceIndex::new(self.0), self.1)
    }

    pub(super) fn metadata(self) -> MetadataRange {
        TableRange::new(MetadataIndex::new(self.0), self.1)
    }
}

fn collect<T, U>(values: Vec<T>) -> Result<Vec<U>, CompiledAssetDecodeError>
where
    U: TryFrom<T, Error = CompiledAssetDecodeError>,
{
    values.into_iter().map(TryInto::try_into).collect()
}
