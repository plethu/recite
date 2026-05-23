use std::fmt;
use std::io::Cursor;
use std::ops::Range;

use serde::Deserialize;
use serde::de::{self, IgnoredAny};
use serde_bytes::ByteBuf;

use crate::{
    BlockId, ChoiceId, CoreValueError, EffectId, LineId, ScalarValue, SourcePosition, SourceSpan,
    SpeakerId, Value,
};

use super::{
    BlockIndex, BlockLookupEntry, BlockLookupTable, COMPILED_ASSET_FORMAT_VERSION_V0,
    COMPILER_COMPATIBILITY_VERSION_V0, ChoiceIndex, ChoiceLookupEntry, ChoiceLookupTable,
    ChoiceRange, CompiledArgument, CompiledAssetEncoding, CompiledAssetHeader, CompiledAssetId,
    CompiledBlock, CompiledChoice, CompiledChoiceEcho, CompiledConditionCall,
    CompiledConditionExpression, CompiledDialogue, CompiledDivertTarget, CompiledEffect,
    CompiledEffectMode, CompiledInspectionEncoding, CompiledLine, CompiledMatchArm,
    CompiledMatchPattern, CompiledMetadataEntry, CompiledSourceFile, CompiledSourceMapEntry,
    CompiledSpeaker, CompiledStatement, CompiledStatementKind, CompiledValueError, CompilerVersion,
    ContentFingerprint, EffectIndex, FingerprintAlgorithm, FingerprintDigest, LineIndex,
    LineLookupEntry, LineLookupTable, MatchArmIndex, MatchArmRange, MetadataIndex, MetadataRange,
    SchemaFingerprint, SourceFileIndex, SourceMapId, SourceMapIndex, SpeakerIndex, StatementIndex,
    StatementRange, TableRange, V0_ARGUMENT_TAG_IDENTIFIER, V0_ARGUMENT_TAG_VALUE,
    V0_ASSET_ENCODING_MESSAGEPACK, V0_CHOICE_ECHO_TAG_EXPLICIT_LINE, V0_CHOICE_ECHO_TAG_NONE,
    V0_CHOICE_ECHO_TAG_SELECTED_TEXT, V0_CONDITION_TAG_AND, V0_CONDITION_TAG_CALL,
    V0_CONDITION_TAG_NOT, V0_CONDITION_TAG_OR, V0_DIVERT_TARGET_TAG_BLOCK,
    V0_DIVERT_TARGET_TAG_END, V0_EFFECT_MODE_TAG_BLOCKING, V0_EFFECT_MODE_TAG_DEFERRED,
    V0_EFFECT_MODE_TAG_IMMEDIATE, V0_INSPECTION_ENCODING_COMPACT_JSON,
    V0_MATCH_PATTERN_TAG_VARIANT, V0_MATCH_PATTERN_TAG_WILDCARD, V0_SCALAR_TAG_BOOLEAN,
    V0_SCALAR_TAG_FLOAT, V0_SCALAR_TAG_INTEGER, V0_SCALAR_TAG_STRING,
    V0_SCHEMA_FINGERPRINT_TAG_FINGERPRINT, V0_SCHEMA_FINGERPRINT_TAG_NO_SCHEMA,
    V0_STATEMENT_TAG_DIVERT, V0_STATEMENT_TAG_EFFECT, V0_STATEMENT_TAG_END, V0_STATEMENT_TAG_IF,
    V0_STATEMENT_TAG_LINE, V0_STATEMENT_TAG_MATCH, V0_STATEMENT_TAG_PROMPT, V0_VALUE_TAG_ARRAY,
    V0_VALUE_TAG_SCALAR,
};

/// Error returned when decoding public v0 compiled dialogue asset bytes.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum CompiledAssetDecodeError {
    UnsupportedFormat {
        format_version: u16,
        compiler_compatibility_version: u16,
    },
    MalformedAsset(String),
}

impl fmt::Display for CompiledAssetDecodeError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::UnsupportedFormat {
                format_version,
                compiler_compatibility_version,
            } => write!(
                formatter,
                "unsupported compiled asset format {format_version} with compatibility version {compiler_compatibility_version}"
            ),
            Self::MalformedAsset(reason) => write!(formatter, "malformed compiled asset: {reason}"),
        }
    }
}

impl std::error::Error for CompiledAssetDecodeError {}

/// Decode deterministic v0 `.recitec` MessagePack bytes into a compiled dialogue asset.
pub fn decode_compiled_dialogue_messagepack(
    bytes: &[u8],
) -> Result<CompiledDialogue, CompiledAssetDecodeError> {
    if let Some((format_version, compiler_compatibility_version)) =
        messagepack_asset_format_versions(bytes)
        && (format_version != COMPILED_ASSET_FORMAT_VERSION_V0
            || compiler_compatibility_version != COMPILER_COMPATIBILITY_VERSION_V0)
    {
        return Err(CompiledAssetDecodeError::UnsupportedFormat {
            format_version,
            compiler_compatibility_version,
        });
    }

    let mut cursor = Cursor::new(bytes);
    let mut deserializer = rmp_serde::Deserializer::new(&mut cursor);
    let wire = MsgDialogue::deserialize(&mut deserializer)
        .map_err(|error| malformed(error.to_string()))?;
    let consumed = cursor.position() as usize;
    if consumed != bytes.len() {
        return Err(malformed(format!(
            "MessagePack asset has {} trailing bytes",
            bytes.len() - consumed
        )));
    }

    let dialogue = wire.try_into()?;
    validate_dialogue(&dialogue)?;
    Ok(dialogue)
}

#[derive(Deserialize)]
struct MsgDialogue(
    MsgHeader,
    u32,
    Vec<MsgSourceFile>,
    Vec<MsgBlock>,
    Vec<MsgStatement>,
    Vec<MsgMatchArm>,
    Vec<MsgLine>,
    Vec<MsgChoice>,
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
            speakers: collect(value.8)?,
            metadata: collect(value.9)?,
            effects: collect(value.10)?,
            source_maps: collect(value.11)?,
            block_lookup: BlockLookupTable::new(
                value
                    .12
                    .into_iter()
                    .map(|entry| entry.block())
                    .collect::<Result<Vec<_>, _>>()?,
            )?,
            line_lookup: LineLookupTable::new(
                value
                    .13
                    .into_iter()
                    .map(|entry| entry.line())
                    .collect::<Result<Vec<_>, _>>()?,
            )?,
            choice_lookup: ChoiceLookupTable::new(
                value
                    .14
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
            condition: value.3.map(|condition| condition.0),
            target: value.4.0,
            echo: value.5.0,
            source_map: SourceMapIndex::new(value.6),
        })
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
struct MsgRange(u32, u32);

impl MsgRange {
    fn statement(self) -> StatementRange {
        TableRange::new(StatementIndex::new(self.0), self.1)
    }

    fn match_arm(self) -> MatchArmRange {
        TableRange::new(MatchArmIndex::new(self.0), self.1)
    }

    fn choice(self) -> ChoiceRange {
        TableRange::new(ChoiceIndex::new(self.0), self.1)
    }

    fn metadata(self) -> MetadataRange {
        TableRange::new(MetadataIndex::new(self.0), self.1)
    }
}

struct MsgAssetEncoding(CompiledAssetEncoding);
struct MsgInspectionEncoding(CompiledInspectionEncoding);
struct MsgSchemaFingerprint(SchemaFingerprint);
struct MsgFingerprint(ContentFingerprint);
struct MsgStatementKind(CompiledStatementKind);
struct MsgMatchPattern(CompiledMatchPattern);
struct MsgDivertTarget(CompiledDivertTarget);
struct MsgChoiceEcho(CompiledChoiceEcho);
struct MsgEffectMode(CompiledEffectMode);
struct MsgConditionExpression(CompiledConditionExpression);
struct MsgArgument(CompiledArgument);
struct MsgValue(Value);
struct MsgScalarValue(ScalarValue);
struct MsgSourceSpan(SourceSpan);

impl<'de> Deserialize<'de> for MsgAssetEncoding {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (tag, payload): (u8, Option<IgnoredAny>) = Deserialize::deserialize(deserializer)?;
        ensure_nil_payload("asset encoding", payload)?;
        match tag {
            V0_ASSET_ENCODING_MESSAGEPACK => Ok(Self(CompiledAssetEncoding::MessagePack)),
            _ => Err(unknown_tag("asset encoding", tag)),
        }
    }
}

impl<'de> Deserialize<'de> for MsgInspectionEncoding {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (tag, payload): (u8, Option<IgnoredAny>) = Deserialize::deserialize(deserializer)?;
        ensure_nil_payload("inspection encoding", payload)?;
        match tag {
            V0_INSPECTION_ENCODING_COMPACT_JSON => {
                Ok(Self(CompiledInspectionEncoding::CompactJson))
            }
            _ => Err(unknown_tag("inspection encoding", tag)),
        }
    }
}

impl<'de> Deserialize<'de> for MsgSchemaFingerprint {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (tag, payload): (u8, Option<MsgFingerprint>) = Deserialize::deserialize(deserializer)?;
        match (tag, payload) {
            (V0_SCHEMA_FINGERPRINT_TAG_FINGERPRINT, Some(fingerprint)) => {
                Ok(Self(SchemaFingerprint::Fingerprint(fingerprint.0)))
            }
            (V0_SCHEMA_FINGERPRINT_TAG_NO_SCHEMA, None) => Ok(Self(SchemaFingerprint::NoSchema)),
            (V0_SCHEMA_FINGERPRINT_TAG_NO_SCHEMA, Some(_)) => {
                Err(unexpected_payload("schema fingerprint"))
            }
            (V0_SCHEMA_FINGERPRINT_TAG_FINGERPRINT, None) => {
                Err(de::Error::custom(CompiledAssetDecodeError::MalformedAsset(
                    "schema fingerprint tag requires fingerprint payload".to_owned(),
                )))
            }
            _ => Err(unknown_tag("schema fingerprint", tag)),
        }
    }
}

impl<'de> Deserialize<'de> for MsgFingerprint {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (algorithm, digest): (String, ByteBuf) = Deserialize::deserialize(deserializer)?;
        let fingerprint = ContentFingerprint::new(
            FingerprintAlgorithm::new(algorithm).map_err(de::Error::custom)?,
            FingerprintDigest::new(digest.into_vec()).map_err(de::Error::custom)?,
        )
        .map_err(de::Error::custom)?;
        Ok(Self(fingerprint))
    }
}

impl<'de> Deserialize<'de> for MsgStatementKind {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (tag, payload): (u8, serde_value::Value) = Deserialize::deserialize(deserializer)?;
        let kind = match tag {
            V0_STATEMENT_TAG_LINE => {
                CompiledStatementKind::Line(LineIndex::new(from_value("line statement", payload)?))
            }
            V0_STATEMENT_TAG_PROMPT => {
                let (line, choices): (Option<u32>, MsgRange) =
                    from_value("prompt statement", payload)?;
                CompiledStatementKind::Prompt {
                    line: line.map(LineIndex::new),
                    choices: choices.choice(),
                }
            }
            V0_STATEMENT_TAG_DIVERT => CompiledStatementKind::Divert(
                from_value::<MsgDivertTarget, D::Error>("divert statement", payload)?.0,
            ),
            V0_STATEMENT_TAG_IF => {
                let (condition, then_statements, else_statements): (
                    MsgConditionExpression,
                    MsgRange,
                    MsgRange,
                ) = from_value("if statement", payload)?;
                CompiledStatementKind::If {
                    condition: condition.0,
                    then_statements: then_statements.statement(),
                    else_statements: else_statements.statement(),
                }
            }
            V0_STATEMENT_TAG_MATCH => {
                let (scrutinee, arms): (MsgConditionCall, MsgRange) =
                    from_value("match statement", payload)?;
                CompiledStatementKind::Match {
                    scrutinee: scrutinee.into_inner().map_err(de::Error::custom)?,
                    arms: arms.match_arm(),
                }
            }
            V0_STATEMENT_TAG_EFFECT => CompiledStatementKind::Effect(EffectIndex::new(from_value(
                "effect statement",
                payload,
            )?)),
            V0_STATEMENT_TAG_END => {
                ensure_nil_value("end statement", payload)?;
                CompiledStatementKind::End
            }
            _ => return Err(unknown_tag("statement kind", tag)),
        };
        Ok(Self(kind))
    }
}

#[derive(Deserialize)]
struct MsgConditionCall(String, Vec<MsgArgument>);

impl MsgConditionCall {
    fn into_inner(self) -> Result<CompiledConditionCall, CompiledAssetDecodeError> {
        ensure_identifier_like("condition function", &self.0)?;
        Ok(CompiledConditionCall {
            function: self.0,
            args: collect_wrapped(self.1),
        })
    }
}

impl<'de> Deserialize<'de> for MsgMatchPattern {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (tag, payload): (u8, serde_value::Value) = Deserialize::deserialize(deserializer)?;
        match tag {
            V0_MATCH_PATTERN_TAG_VARIANT => Ok(Self(CompiledMatchPattern::Variant(from_value(
                "match pattern",
                payload,
            )?))),
            V0_MATCH_PATTERN_TAG_WILDCARD => {
                ensure_nil_value("match wildcard", payload)?;
                Ok(Self(CompiledMatchPattern::Wildcard))
            }
            _ => Err(unknown_tag("match pattern", tag)),
        }
    }
}

impl<'de> Deserialize<'de> for MsgDivertTarget {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (tag, payload): (u8, serde_value::Value) = Deserialize::deserialize(deserializer)?;
        match tag {
            V0_DIVERT_TARGET_TAG_BLOCK => Ok(Self(CompiledDivertTarget::Block(BlockIndex::new(
                from_value("block divert", payload)?,
            )))),
            V0_DIVERT_TARGET_TAG_END => {
                ensure_nil_value("end divert", payload)?;
                Ok(Self(CompiledDivertTarget::End))
            }
            _ => Err(unknown_tag("divert target", tag)),
        }
    }
}

impl<'de> Deserialize<'de> for MsgChoiceEcho {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (tag, payload): (u8, serde_value::Value) = Deserialize::deserialize(deserializer)?;
        match tag {
            V0_CHOICE_ECHO_TAG_NONE => {
                ensure_nil_value("choice echo none", payload)?;
                Ok(Self(CompiledChoiceEcho::None))
            }
            V0_CHOICE_ECHO_TAG_SELECTED_TEXT => {
                ensure_nil_value("choice echo selected text", payload)?;
                Ok(Self(CompiledChoiceEcho::SelectedText))
            }
            V0_CHOICE_ECHO_TAG_EXPLICIT_LINE => Ok(Self(CompiledChoiceEcho::ExplicitLine(
                LineId::new(from_value::<String, D::Error>(
                    "choice echo explicit line",
                    payload,
                )?)
                .map_err(de::Error::custom)?,
            ))),
            _ => Err(unknown_tag("choice echo", tag)),
        }
    }
}

impl<'de> Deserialize<'de> for MsgEffectMode {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (tag, payload): (u8, Option<IgnoredAny>) = Deserialize::deserialize(deserializer)?;
        ensure_nil_payload("effect mode", payload)?;
        match tag {
            V0_EFFECT_MODE_TAG_DEFERRED => Ok(Self(CompiledEffectMode::Deferred)),
            V0_EFFECT_MODE_TAG_IMMEDIATE => Ok(Self(CompiledEffectMode::Immediate)),
            V0_EFFECT_MODE_TAG_BLOCKING => Ok(Self(CompiledEffectMode::Blocking)),
            _ => Err(unknown_tag("effect mode", tag)),
        }
    }
}

impl<'de> Deserialize<'de> for MsgConditionExpression {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (tag, payload): (u8, serde_value::Value) = Deserialize::deserialize(deserializer)?;
        match tag {
            V0_CONDITION_TAG_CALL => Ok(Self(CompiledConditionExpression::Call(
                from_value::<MsgConditionCall, D::Error>("condition call", payload)?
                    .into_inner()
                    .map_err(de::Error::custom)?,
            ))),
            V0_CONDITION_TAG_AND => Ok(Self(CompiledConditionExpression::And(collect_wrapped(
                from_value::<Vec<MsgConditionExpression>, D::Error>("condition and", payload)?,
            )))),
            V0_CONDITION_TAG_OR => Ok(Self(CompiledConditionExpression::Or(collect_wrapped(
                from_value::<Vec<MsgConditionExpression>, D::Error>("condition or", payload)?,
            )))),
            V0_CONDITION_TAG_NOT => Ok(Self(CompiledConditionExpression::Not(Box::new(
                from_value::<MsgConditionExpression, D::Error>("condition not", payload)?.0,
            )))),
            _ => Err(unknown_tag("condition expression", tag)),
        }
    }
}

impl<'de> Deserialize<'de> for MsgArgument {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (tag, payload): (u8, serde_value::Value) = Deserialize::deserialize(deserializer)?;
        match tag {
            V0_ARGUMENT_TAG_IDENTIFIER => {
                let value: String = from_value("argument identifier", payload)?;
                ensure_identifier_like("argument identifier", &value).map_err(de::Error::custom)?;
                Ok(Self(CompiledArgument::Identifier(value)))
            }
            V0_ARGUMENT_TAG_VALUE => Ok(Self(CompiledArgument::Value(
                from_value::<MsgScalarValue, D::Error>("argument value", payload)?.0,
            ))),
            _ => Err(unknown_tag("argument", tag)),
        }
    }
}

impl<'de> Deserialize<'de> for MsgValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (tag, payload): (u8, serde_value::Value) = Deserialize::deserialize(deserializer)?;
        match tag {
            V0_VALUE_TAG_SCALAR => Ok(Self(Value::Scalar(
                from_value::<MsgScalarValue, D::Error>("scalar value", payload)?.0,
            ))),
            V0_VALUE_TAG_ARRAY => Ok(Self(Value::Array(collect_wrapped(from_value::<
                Vec<MsgScalarValue>,
                D::Error,
            >(
                "array value",
                payload,
            )?)))),
            _ => Err(unknown_tag("value", tag)),
        }
    }
}

impl<'de> Deserialize<'de> for MsgScalarValue {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (tag, payload): (u8, serde_value::Value) = Deserialize::deserialize(deserializer)?;
        match tag {
            V0_SCALAR_TAG_STRING => Ok(Self(ScalarValue::String(from_value(
                "string scalar",
                payload,
            )?))),
            V0_SCALAR_TAG_INTEGER => Ok(Self(ScalarValue::Integer(from_value(
                "integer scalar",
                payload,
            )?))),
            V0_SCALAR_TAG_FLOAT => {
                let value = from_value("float scalar", payload)?;
                if !f64::is_finite(value) {
                    return Err(de::Error::custom(CompiledAssetDecodeError::MalformedAsset(
                        "float scalar must be finite".to_owned(),
                    )));
                }
                Ok(Self(ScalarValue::Float(value)))
            }
            V0_SCALAR_TAG_BOOLEAN => Ok(Self(ScalarValue::Boolean(from_value(
                "boolean scalar",
                payload,
            )?))),
            _ => Err(unknown_tag("scalar value", tag)),
        }
    }
}

impl<'de> Deserialize<'de> for MsgSourceSpan {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        let (file, start_line, start_column, end_line, end_column): (
            String,
            u32,
            u32,
            Option<u32>,
            Option<u32>,
        ) = Deserialize::deserialize(deserializer)?;
        let start = SourcePosition::new(start_line, start_column).map_err(de::Error::custom)?;
        let end = match (end_line, end_column) {
            (Some(line), Some(column)) => {
                Some(SourcePosition::new(line, column).map_err(de::Error::custom)?)
            }
            (None, None) => None,
            _ => {
                return Err(de::Error::custom(CompiledAssetDecodeError::MalformedAsset(
                    "source span end line and column must both be present or both be nil"
                        .to_owned(),
                )));
            }
        };
        if end.is_some_and(|end| end < start) {
            return Err(de::Error::custom(CompiledAssetDecodeError::MalformedAsset(
                "source span end precedes span start".to_owned(),
            )));
        }
        Ok(Self(SourceSpan::new(file, start, end)))
    }
}

fn validate_dialogue(dialogue: &CompiledDialogue) -> Result<(), CompiledAssetDecodeError> {
    let block_len = dialogue.blocks.len();
    ensure_index("default block", block_len, dialogue.default_block.as_u32())?;

    for (index, source) in dialogue.sources.iter().enumerate() {
        if source.path.is_empty() {
            return Err(malformed(format!(
                "source file {index} path must not be empty"
            )));
        }
    }
    ensure_unique_strings(
        "source file path",
        dialogue.sources.iter().map(|source| source.path.as_str()),
    )?;

    for block in &dialogue.blocks {
        ensure_index(
            "block source file",
            dialogue.sources.len(),
            block.source_file.as_u32(),
        )?;
        ensure_range(
            "block statements",
            dialogue.statements.len(),
            block.statements,
            StatementIndex::as_u32,
        )?;
        ensure_range(
            "block metadata",
            dialogue.metadata.len(),
            block.metadata,
            MetadataIndex::as_u32,
        )?;
        if let Some(speaker) = block.default_speaker {
            ensure_index(
                "block default speaker",
                dialogue.speakers.len(),
                speaker.as_u32(),
            )?;
        }
        ensure_index(
            "block source map",
            dialogue.source_maps.len(),
            block.source_map.as_u32(),
        )?;
    }

    for statement in &dialogue.statements {
        validate_statement(dialogue, statement)?;
    }
    for arm in &dialogue.match_arms {
        ensure_range(
            "match arm statements",
            dialogue.statements.len(),
            arm.statements,
            StatementIndex::as_u32,
        )?;
        ensure_index(
            "match arm source map",
            dialogue.source_maps.len(),
            arm.source_map.as_u32(),
        )?;
    }
    for line in &dialogue.lines {
        if let Some(speaker) = line.speaker {
            ensure_index("line speaker", dialogue.speakers.len(), speaker.as_u32())?;
        }
        ensure_range(
            "line metadata",
            dialogue.metadata.len(),
            line.metadata,
            MetadataIndex::as_u32,
        )?;
        ensure_index(
            "line source map",
            dialogue.source_maps.len(),
            line.source_map.as_u32(),
        )?;
    }
    for choice in &dialogue.choices {
        ensure_range(
            "choice metadata",
            dialogue.metadata.len(),
            choice.metadata,
            MetadataIndex::as_u32,
        )?;
        if let Some(condition) = &choice.condition {
            validate_condition(condition)?;
        }
        validate_divert(dialogue, &choice.target)?;
        validate_choice_echo(dialogue, &choice.echo)?;
        ensure_index(
            "choice source map",
            dialogue.source_maps.len(),
            choice.source_map.as_u32(),
        )?;
    }
    for metadata in &dialogue.metadata {
        if let Some(source_map) = metadata.source_map {
            ensure_index(
                "metadata source map",
                dialogue.source_maps.len(),
                source_map.as_u32(),
            )?;
        }
    }
    for effect in &dialogue.effects {
        ensure_index(
            "effect source map",
            dialogue.source_maps.len(),
            effect.source_map.as_u32(),
        )?;
    }
    ensure_unique_strings(
        "effect id",
        dialogue.effects.iter().map(|effect| effect.id.as_str()),
    )?;
    for source_map in &dialogue.source_maps {
        ensure_index(
            "source map source file",
            dialogue.sources.len(),
            source_map.source_file.as_u32(),
        )?;
        let source = &dialogue.sources[source_map.source_file.as_u32() as usize];
        if source.path != source_map.span.file {
            return Err(malformed(format!(
                "source map span file `{}` does not match source file `{}`",
                source_map.span.file, source.path
            )));
        }
    }
    validate_lookup_entries(
        "block lookup",
        dialogue
            .blocks
            .iter()
            .map(|block| block.id.as_str())
            .collect(),
        dialogue
            .block_lookup
            .as_slice()
            .iter()
            .map(|entry| (entry.id.as_str(), entry.index.as_u32())),
    )?;
    validate_lookup_entries(
        "line lookup",
        dialogue.lines.iter().map(|line| line.id.as_str()).collect(),
        dialogue
            .line_lookup
            .as_slice()
            .iter()
            .map(|entry| (entry.id.as_str(), entry.index.as_u32())),
    )?;
    validate_lookup_entries(
        "choice lookup",
        dialogue
            .choices
            .iter()
            .map(|choice| choice.id.as_str())
            .collect(),
        dialogue
            .choice_lookup
            .as_slice()
            .iter()
            .map(|entry| (entry.id.as_str(), entry.index.as_u32())),
    )?;
    validate_disjoint_ids(
        "line and choice ids",
        dialogue.lines.iter().map(|line| line.id.as_str()),
        dialogue.choices.iter().map(|choice| choice.id.as_str()),
    )?;

    Ok(())
}

fn validate_statement(
    dialogue: &CompiledDialogue,
    statement: &CompiledStatement,
) -> Result<(), CompiledAssetDecodeError> {
    match &statement.kind {
        CompiledStatementKind::Line(index) => {
            ensure_index("line statement", dialogue.lines.len(), index.as_u32())?;
        }
        CompiledStatementKind::Prompt { line, choices } => {
            if let Some(index) = line {
                ensure_index("prompt line", dialogue.lines.len(), index.as_u32())?;
            }
            if choices.len == 0 {
                return Err(malformed("prompt choices must not be empty".to_owned()));
            }
            ensure_range(
                "prompt choices",
                dialogue.choices.len(),
                *choices,
                ChoiceIndex::as_u32,
            )?;
        }
        CompiledStatementKind::Divert(target) => validate_divert(dialogue, target)?,
        CompiledStatementKind::If {
            condition,
            then_statements,
            else_statements,
        } => {
            validate_condition(condition)?;
            ensure_range(
                "if then statements",
                dialogue.statements.len(),
                *then_statements,
                StatementIndex::as_u32,
            )?;
            ensure_range(
                "if else statements",
                dialogue.statements.len(),
                *else_statements,
                StatementIndex::as_u32,
            )?;
        }
        CompiledStatementKind::Match { scrutinee, arms } => {
            for argument in &scrutinee.args {
                validate_argument(argument)?;
            }
            ensure_range(
                "match arms",
                dialogue.match_arms.len(),
                *arms,
                MatchArmIndex::as_u32,
            )?;
        }
        CompiledStatementKind::Effect(index) => {
            ensure_index("effect statement", dialogue.effects.len(), index.as_u32())?;
        }
        CompiledStatementKind::End => {}
    }
    ensure_index(
        "statement source map",
        dialogue.source_maps.len(),
        statement.source_map.as_u32(),
    )?;
    Ok(())
}

fn validate_divert(
    dialogue: &CompiledDialogue,
    target: &CompiledDivertTarget,
) -> Result<(), CompiledAssetDecodeError> {
    if let CompiledDivertTarget::Block(index) = target {
        ensure_index("block divert", dialogue.blocks.len(), index.as_u32())?;
    }
    Ok(())
}

fn validate_condition(
    condition: &CompiledConditionExpression,
) -> Result<(), CompiledAssetDecodeError> {
    match condition {
        CompiledConditionExpression::Call(call) => {
            for argument in &call.args {
                validate_argument(argument)?;
            }
        }
        CompiledConditionExpression::And(expressions) => {
            if expressions.is_empty() {
                return Err(malformed(
                    "condition and group must not be empty".to_owned(),
                ));
            }
            for expression in expressions {
                validate_condition(expression)?;
            }
        }
        CompiledConditionExpression::Or(expressions) => {
            if expressions.is_empty() {
                return Err(malformed("condition or group must not be empty".to_owned()));
            }
            for expression in expressions {
                validate_condition(expression)?;
            }
        }
        CompiledConditionExpression::Not(expression) => validate_condition(expression)?,
    }
    Ok(())
}

fn validate_argument(argument: &CompiledArgument) -> Result<(), CompiledAssetDecodeError> {
    if let CompiledArgument::Value(ScalarValue::Float(value)) = argument
        && !value.is_finite()
    {
        return Err(malformed("float scalar must be finite".to_owned()));
    }
    Ok(())
}

fn validate_choice_echo(
    dialogue: &CompiledDialogue,
    echo: &CompiledChoiceEcho,
) -> Result<(), CompiledAssetDecodeError> {
    let CompiledChoiceEcho::ExplicitLine(line_id) = echo else {
        return Ok(());
    };
    if dialogue
        .line_lookup
        .as_slice()
        .binary_search_by(|entry| entry.id.as_str().cmp(line_id.as_str()))
        .is_ok()
    {
        return Ok(());
    }
    Err(malformed(format!(
        "choice echo references unknown line id `{line_id}`"
    )))
}

fn messagepack_asset_format_versions(bytes: &[u8]) -> Option<(u16, u16)> {
    let (item_count, mut offset) = read_array_len(bytes)?;
    if item_count == 0 {
        return None;
    }
    let (header_count, header_offset) = read_array_len(&bytes[offset..])?;
    if header_count < 2 {
        return None;
    }
    offset += header_offset;

    let format_version = read_u16(bytes, &mut offset)?;
    let compiler_compatibility_version = read_u16(bytes, &mut offset)?;
    Some((format_version, compiler_compatibility_version))
}

fn read_array_len(bytes: &[u8]) -> Option<(u32, usize)> {
    let marker = *bytes.first()?;
    match marker {
        0x90..=0x9f => Some((u32::from(marker & 0x0f), 1)),
        0xdc => Some((u32::from(u16::from_be_bytes(read_array(bytes, 1)?)), 3)),
        0xdd => Some((u32::from_be_bytes(read_array(bytes, 1)?), 5)),
        _ => None,
    }
}

fn read_u16(bytes: &[u8], offset: &mut usize) -> Option<u16> {
    let marker = *bytes.get(*offset)?;
    *offset += 1;

    match marker {
        0x00..=0x7f => Some(u16::from(marker)),
        0xcc => {
            let value = *bytes.get(*offset)?;
            *offset += 1;
            Some(u16::from(value))
        }
        0xcd => {
            let value = u16::from_be_bytes(read_array(bytes, *offset)?);
            *offset += 2;
            Some(value)
        }
        _ => None,
    }
}

fn read_array<const N: usize>(bytes: &[u8], offset: usize) -> Option<[u8; N]> {
    bytes.get(offset..offset + N)?.try_into().ok()
}

fn ensure_identifier_like(
    field: &'static str,
    value: &str,
) -> Result<(), CompiledAssetDecodeError> {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return Err(malformed(format!("{field} must not be empty")));
    };
    if (first.is_ascii_alphabetic() || first == '_')
        && chars.all(|character| {
            character.is_ascii_alphanumeric()
                || character == '_'
                || character == '.'
                || character == '-'
        })
    {
        Ok(())
    } else {
        Err(malformed(format!(
            "{field} must be an identifier-like name"
        )))
    }
}

fn ensure_non_empty(field: &'static str, value: &str) -> Result<(), CompiledAssetDecodeError> {
    if value.is_empty() {
        Err(malformed(format!("{field} must not be empty")))
    } else {
        Ok(())
    }
}

fn ensure_unique_strings<'a>(
    field: &'static str,
    values: impl IntoIterator<Item = &'a str>,
) -> Result<(), CompiledAssetDecodeError> {
    let mut values = values.into_iter().collect::<Vec<_>>();
    values.sort_unstable();
    for window in values.windows(2) {
        if window[0] == window[1] {
            return Err(malformed(format!(
                "{field} `{}` appears more than once",
                window[0]
            )));
        }
    }
    Ok(())
}

fn validate_disjoint_ids<'a>(
    field: &'static str,
    left: impl IntoIterator<Item = &'a str>,
    right: impl IntoIterator<Item = &'a str>,
) -> Result<(), CompiledAssetDecodeError> {
    let mut left = left.into_iter().collect::<Vec<_>>();
    let mut right = right.into_iter().collect::<Vec<_>>();
    left.sort_unstable();
    right.sort_unstable();

    let mut left_index = 0;
    let mut right_index = 0;
    while left_index < left.len() && right_index < right.len() {
        match left[left_index].cmp(right[right_index]) {
            std::cmp::Ordering::Less => left_index += 1,
            std::cmp::Ordering::Greater => right_index += 1,
            std::cmp::Ordering::Equal => {
                return Err(malformed(format!(
                    "{field} must be unique, got duplicate `{}`",
                    left[left_index]
                )));
            }
        }
    }
    Ok(())
}

fn validate_lookup_entries<'a>(
    table: &'static str,
    row_ids: Vec<&'a str>,
    entries: impl IntoIterator<Item = (&'a str, u32)>,
) -> Result<(), CompiledAssetDecodeError> {
    let entries = entries.into_iter().collect::<Vec<_>>();
    if entries.len() != row_ids.len() {
        return Err(malformed(format!(
            "{table} has {} entries for {} table rows",
            entries.len(),
            row_ids.len()
        )));
    }

    for (id, index) in entries {
        let Some(row_id) = row_ids.get(index as usize) else {
            return Err(malformed(format!(
                "{table} index {index} is out of range for table length {}",
                row_ids.len()
            )));
        };
        if *row_id != id {
            return Err(malformed(format!(
                "{table} entry `{id}` points to row `{row_id}` at index {index}"
            )));
        }
    }
    Ok(())
}

fn ensure_index(
    field: &'static str,
    table_len: usize,
    index: u32,
) -> Result<(), CompiledAssetDecodeError> {
    if (index as usize) < table_len {
        Ok(())
    } else {
        Err(malformed(format!(
            "{field} index {index} is out of range for table length {table_len}"
        )))
    }
}

fn ensure_range<I: Copy>(
    field: &'static str,
    table_len: usize,
    range: TableRange<I>,
    index: impl Fn(I) -> u32,
) -> Result<Range<usize>, CompiledAssetDecodeError> {
    let start = index(range.start) as usize;
    let len = range.len as usize;
    let end = start
        .checked_add(len)
        .ok_or_else(|| malformed(format!("{field} range overflows usize")))?;

    if end > table_len {
        return Err(malformed(format!(
            "{field} range {start}..{end} exceeds table length {table_len}"
        )));
    }

    Ok(start..end)
}

fn collect<T, U>(values: Vec<T>) -> Result<Vec<U>, CompiledAssetDecodeError>
where
    U: TryFrom<T, Error = CompiledAssetDecodeError>,
{
    values.into_iter().map(U::try_from).collect()
}

fn collect_wrapped<T, U>(values: Vec<T>) -> Vec<U>
where
    T: IntoWrapped<U>,
{
    values.into_iter().map(IntoWrapped::into_wrapped).collect()
}

trait IntoWrapped<T> {
    fn into_wrapped(self) -> T;
}

impl IntoWrapped<CompiledArgument> for MsgArgument {
    fn into_wrapped(self) -> CompiledArgument {
        self.0
    }
}

impl IntoWrapped<CompiledConditionExpression> for MsgConditionExpression {
    fn into_wrapped(self) -> CompiledConditionExpression {
        self.0
    }
}

impl IntoWrapped<ScalarValue> for MsgScalarValue {
    fn into_wrapped(self) -> ScalarValue {
        self.0
    }
}

fn ensure_nil_payload<E>(field: &'static str, payload: Option<IgnoredAny>) -> Result<(), E>
where
    E: de::Error,
{
    if payload.is_none() {
        Ok(())
    } else {
        Err(unexpected_payload(field))
    }
}

fn ensure_nil_value<E>(field: &'static str, payload: serde_value::Value) -> Result<(), E>
where
    E: de::Error,
{
    if matches!(
        payload,
        serde_value::Value::Unit | serde_value::Value::Option(None)
    ) {
        Ok(())
    } else {
        Err(unexpected_payload(field))
    }
}

fn from_value<T, E>(field: &'static str, value: serde_value::Value) -> Result<T, E>
where
    T: for<'de> Deserialize<'de>,
    E: de::Error,
{
    T::deserialize(value).map_err(|error| {
        de::Error::custom(CompiledAssetDecodeError::MalformedAsset(format!(
            "{field} payload is malformed: {error}"
        )))
    })
}

fn unknown_tag<E>(field: &'static str, tag: u8) -> E
where
    E: de::Error,
{
    de::Error::custom(format!("unknown {field} tag {tag}"))
}

fn unexpected_payload<E>(field: &'static str) -> E
where
    E: de::Error,
{
    de::Error::custom(format!("{field} tag must have nil payload"))
}

fn malformed(reason: String) -> CompiledAssetDecodeError {
    CompiledAssetDecodeError::MalformedAsset(reason)
}

impl From<CompiledValueError> for CompiledAssetDecodeError {
    fn from(error: CompiledValueError) -> Self {
        malformed(error.to_string())
    }
}

impl From<CoreValueError> for CompiledAssetDecodeError {
    fn from(error: CoreValueError) -> Self {
        malformed(error.to_string())
    }
}
