use serde::Deserialize;

use crate::{AvailabilityReasonId, BlockId, ChoiceId, EffectId, LineId, SpeakerId};

use super::CompiledAssetDecodeError;
use super::tags::{
    MsgArgument, MsgAssetEncoding, MsgChoiceEcho, MsgConditionExpression, MsgDivertTarget,
    MsgEffectMode, MsgFingerprint, MsgInspectionEncoding, MsgMatchPattern, MsgSchemaFingerprint,
    MsgSourceSpan, MsgStatementKind, MsgValue, collect_wrapped, ensure_identifier_like,
};
use crate::compiled::{
    BlockIndex, BlockLookupEntry, BlockLookupTable, COMPILED_ASSET_FORMAT_VERSION_V0,
    COMPILER_COMPATIBILITY_VERSION_V0, ChoiceIndex, ChoiceLookupEntry, ChoiceLookupTable,
    ChoiceRange, CompiledAssetHeader, CompiledAssetId, CompiledBlock, CompiledChoice,
    CompiledDialogue, CompiledEffect, CompiledLine, CompiledMatchArm, CompiledMetadataEntry,
    CompiledSourceFile, CompiledSourceMapEntry, CompiledSpeaker, CompiledStatement,
    CompilerVersion, LineIndex, LineLookupEntry, LineLookupTable, MatchArmIndex, MatchArmRange,
    MetadataIndex, MetadataRange, SourceFileIndex, SourceMapId, SourceMapIndex, SpeakerIndex,
    StatementIndex, StatementRange, TableRange,
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

#[derive(Deserialize)]
struct MsgLine(String, String, Option<u32>, MsgRange, u32);

impl TryFrom<MsgLine> for CompiledLine {
    type Error = CompiledAssetDecodeError;

    fn try_from(value: MsgLine) -> Result<Self, Self::Error> {
        Ok(Self {
            id: LineId::new(value.0)?,
            source_text: value.1,
            speaker: value.2.map(SpeakerIndex::new),
            metadata: value.3.metadata(),
            source_map: SourceMapIndex::new(value.4),
        })
    }
}

#[derive(Deserialize)]
struct MsgChoice(
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
        Ok(Self {
            id: ChoiceId::new(value.0)?,
            source_text: value.1,
            metadata: value.2.metadata(),
            availability_requirement: value.3.map(|condition| condition.0),
            availability_requirement_source_text: value.4,
            availability_reason_override: value.5.map(AvailabilityReasonId::new).transpose()?,
            target: value.6.0,
            echo: value.7.0,
            source_map: SourceMapIndex::new(value.8),
        })
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
        ensure_non_empty("condition availability reason function", &value.0)?;
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
        ensure_non_empty("availability reason argument name", &value.0)?;
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
        let (tag, value): (String, String) = Deserialize::deserialize(deserializer)?;
        match tag.as_str() {
            "ConditionArg" => {
                ensure_non_empty("availability reason condition argument", &value)
                    .map_err(serde::de::Error::custom)?;
                Ok(Self(
                    crate::compiled::CompiledAvailabilityReasonArgValue::ConditionArg(value),
                ))
            }
            "Literal" => Ok(Self(
                crate::compiled::CompiledAvailabilityReasonArgValue::Literal(value),
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
        ensure_non_empty("metadata key", &value.0)?;
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
        ensure_identifier_like("effect function", &value.2)?;
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

fn ensure_non_empty(field: &'static str, value: &str) -> Result<(), CompiledAssetDecodeError> {
    if value.is_empty() {
        Err(super::malformed(format!("{field} must not be empty")))
    } else {
        Ok(())
    }
}
