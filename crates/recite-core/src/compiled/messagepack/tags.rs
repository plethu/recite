//! MessagePack v0 tagged-value decoders.
//!
//! This module is the runtime/core decode half of the same wire format encoded
//! by `crates/recite-compiler/src/wire/messagepack/tags.rs`. Both halves are
//! keyed by the shared `V0_*` tag constants defined in
//! `crate::compiled::wire`; add, remove, or renumber tags in all three places
//! together. Once a v0 reader ships, tag changes also require the versioning
//! policy in `docs/recite-production-spec.md` §12.2.

use serde::Deserialize;
use serde::de::{self, IgnoredAny};
use serde_bytes::ByteBuf;

use crate::{LineId, ScalarValue, SourcePosition, SourceSpan, Value};

use super::{CompiledAssetDecodeError, malformed};
use crate::compiled::messagepack::wire::MsgRange;
use crate::compiled::{
    BlockIndex, CompiledArgument, CompiledAssetEncoding, CompiledChoiceEcho, CompiledConditionCall,
    CompiledConditionExpression, CompiledDivertTarget, CompiledEffectMode,
    CompiledInspectionEncoding, CompiledMatchPattern, CompiledStatementKind, ContentFingerprint,
    EffectIndex, FingerprintAlgorithm, FingerprintDigest, LineIndex, SchemaFingerprint,
    V0_ARGUMENT_TAG_IDENTIFIER, V0_ARGUMENT_TAG_VALUE, V0_ASSET_ENCODING_MESSAGEPACK,
    V0_CHOICE_ECHO_TAG_EXPLICIT_LINE, V0_CHOICE_ECHO_TAG_NONE, V0_CHOICE_ECHO_TAG_SELECTED_TEXT,
    V0_CONDITION_TAG_AND, V0_CONDITION_TAG_CALL, V0_CONDITION_TAG_NOT, V0_CONDITION_TAG_OR,
    V0_DIVERT_TARGET_TAG_BLOCK, V0_DIVERT_TARGET_TAG_END, V0_EFFECT_MODE_TAG_BLOCKING,
    V0_EFFECT_MODE_TAG_DEFERRED, V0_EFFECT_MODE_TAG_IMMEDIATE, V0_INSPECTION_ENCODING_COMPACT_JSON,
    V0_MATCH_PATTERN_TAG_VARIANT, V0_MATCH_PATTERN_TAG_WILDCARD, V0_SCALAR_TAG_BOOLEAN,
    V0_SCALAR_TAG_FLOAT, V0_SCALAR_TAG_INTEGER, V0_SCALAR_TAG_STRING,
    V0_SCHEMA_FINGERPRINT_TAG_FINGERPRINT, V0_SCHEMA_FINGERPRINT_TAG_NO_SCHEMA,
    V0_STATEMENT_TAG_DIVERT, V0_STATEMENT_TAG_EFFECT, V0_STATEMENT_TAG_END, V0_STATEMENT_TAG_IF,
    V0_STATEMENT_TAG_LINE, V0_STATEMENT_TAG_MATCH, V0_STATEMENT_TAG_PROMPT, V0_VALUE_TAG_ARRAY,
    V0_VALUE_TAG_SCALAR,
};

pub(super) struct MsgAssetEncoding(pub(super) CompiledAssetEncoding);
pub(super) struct MsgInspectionEncoding(pub(super) CompiledInspectionEncoding);
pub(super) struct MsgSchemaFingerprint(pub(super) SchemaFingerprint);
pub(super) struct MsgFingerprint(pub(super) ContentFingerprint);
pub(super) struct MsgStatementKind(pub(super) CompiledStatementKind);
pub(super) struct MsgMatchPattern(pub(super) CompiledMatchPattern);
pub(super) struct MsgDivertTarget(pub(super) CompiledDivertTarget);
pub(super) struct MsgChoiceEcho(pub(super) CompiledChoiceEcho);
pub(super) struct MsgEffectMode(pub(super) CompiledEffectMode);
pub(super) struct MsgConditionExpression(pub(super) CompiledConditionExpression);
pub(super) struct MsgArgument(pub(super) CompiledArgument);
pub(super) struct MsgValue(pub(super) Value);
struct MsgScalarValue(ScalarValue);
pub(super) struct MsgSourceSpan(pub(super) SourceSpan);

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
pub(super) fn collect_wrapped<T, U>(values: Vec<T>) -> Vec<U>
where
    T: IntoWrapped<U>,
{
    values.into_iter().map(IntoWrapped::into_wrapped).collect()
}

pub(super) fn ensure_identifier_like(
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

pub(super) trait IntoWrapped<T> {
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
