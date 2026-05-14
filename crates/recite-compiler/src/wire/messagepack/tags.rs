use recite_core::{
    CompiledArgument, CompiledAssetEncoding, CompiledChoiceEcho, CompiledConditionCall,
    CompiledConditionExpression, CompiledDivertTarget, CompiledEffectMode,
    CompiledInspectionEncoding, CompiledMatchPattern, CompiledStatementKind, LineIndex,
    ScalarValue, SchemaFingerprint, SourceSpan, Value,
};
use serde::Serialize;
use serde::ser::SerializeTuple;

use super::{choice_range, match_arm_range, statement_range};
use crate::wire::shared::{scalar_value_tag, value_tag};

pub(super) struct MsgAssetEncoding(pub(super) CompiledAssetEncoding);
pub(super) struct MsgInspectionEncoding(pub(super) CompiledInspectionEncoding);
pub(super) struct MsgSchemaFingerprint<'a>(pub(super) &'a SchemaFingerprint);
pub(super) struct MsgFingerprint<'a>(pub(super) &'a recite_core::ContentFingerprint);
pub(super) struct MsgStatementKind<'a>(pub(super) &'a CompiledStatementKind);
pub(super) struct MsgMatchPattern<'a>(pub(super) &'a CompiledMatchPattern);
pub(super) struct MsgDivertTarget<'a>(pub(super) &'a CompiledDivertTarget);
pub(super) struct MsgChoiceEcho<'a>(pub(super) &'a CompiledChoiceEcho);
pub(super) struct MsgEffectMode(pub(super) CompiledEffectMode);
pub(super) struct MsgConditionExpression<'a>(pub(super) &'a CompiledConditionExpression);
pub(super) struct MsgConditionCall<'a>(pub(super) &'a CompiledConditionCall);
pub(super) struct MsgArgument<'a>(pub(super) &'a CompiledArgument);
pub(super) struct MsgValue<'a>(pub(super) &'a Value);
pub(super) struct MsgScalarValue<'a>(pub(super) &'a ScalarValue);
pub(super) struct MsgSourceSpan<'a>(pub(super) &'a SourceSpan);

macro_rules! serialize_tagged {
    ($serializer:expr, $tag:expr, $payload:expr) => {{
        let mut tuple = $serializer.serialize_tuple(2)?;
        tuple.serialize_element(&$tag)?;
        tuple.serialize_element(&$payload)?;
        tuple.end()
    }};
}

impl Serialize for MsgAssetEncoding {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.0 {
            CompiledAssetEncoding::MessagePack => serialize_tagged!(
                serializer,
                recite_core::V0_ASSET_ENCODING_MESSAGEPACK,
                Option::<u8>::None
            ),
        }
    }
}

impl Serialize for MsgInspectionEncoding {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.0 {
            CompiledInspectionEncoding::CompactJson => serialize_tagged!(
                serializer,
                recite_core::V0_INSPECTION_ENCODING_COMPACT_JSON,
                Option::<u8>::None
            ),
        }
    }
}

impl Serialize for MsgSchemaFingerprint<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.0 {
            SchemaFingerprint::Fingerprint(fingerprint) => serialize_tagged!(
                serializer,
                recite_core::V0_SCHEMA_FINGERPRINT_TAG_FINGERPRINT,
                MsgFingerprint(fingerprint)
            ),
            SchemaFingerprint::NoSchema => serialize_tagged!(
                serializer,
                recite_core::V0_SCHEMA_FINGERPRINT_TAG_NO_SCHEMA,
                Option::<u8>::None
            ),
        }
    }
}

impl Serialize for MsgFingerprint<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(self.0.algorithm().as_str())?;
        tuple.serialize_element(serde_bytes::Bytes::new(self.0.digest().as_bytes()))?;
        tuple.end()
    }
}

impl Serialize for MsgStatementKind<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.0 {
            CompiledStatementKind::Line(index) => {
                serialize_tagged!(
                    serializer,
                    recite_core::V0_STATEMENT_TAG_LINE,
                    index.as_u32()
                )
            }
            CompiledStatementKind::Prompt { line, choices } => serialize_tagged!(
                serializer,
                recite_core::V0_STATEMENT_TAG_PROMPT,
                (line.map(LineIndex::as_u32), choice_range(*choices))
            ),
            CompiledStatementKind::Divert(target) => serialize_tagged!(
                serializer,
                recite_core::V0_STATEMENT_TAG_DIVERT,
                MsgDivertTarget(target)
            ),
            CompiledStatementKind::If {
                condition,
                then_statements,
                else_statements,
            } => serialize_tagged!(
                serializer,
                recite_core::V0_STATEMENT_TAG_IF,
                (
                    MsgConditionExpression(condition),
                    statement_range(*then_statements),
                    statement_range(*else_statements)
                )
            ),
            CompiledStatementKind::Match { scrutinee, arms } => serialize_tagged!(
                serializer,
                recite_core::V0_STATEMENT_TAG_MATCH,
                (MsgConditionCall(scrutinee), match_arm_range(*arms))
            ),
            CompiledStatementKind::Effect(index) => serialize_tagged!(
                serializer,
                recite_core::V0_STATEMENT_TAG_EFFECT,
                index.as_u32()
            ),
            CompiledStatementKind::End => serialize_tagged!(
                serializer,
                recite_core::V0_STATEMENT_TAG_END,
                Option::<u8>::None
            ),
        }
    }
}

impl Serialize for MsgMatchPattern<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.0 {
            CompiledMatchPattern::Variant(value) => {
                serialize_tagged!(serializer, recite_core::V0_MATCH_PATTERN_TAG_VARIANT, value)
            }
            CompiledMatchPattern::Wildcard => serialize_tagged!(
                serializer,
                recite_core::V0_MATCH_PATTERN_TAG_WILDCARD,
                Option::<u8>::None
            ),
        }
    }
}

impl Serialize for MsgDivertTarget<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.0 {
            CompiledDivertTarget::Block(index) => serialize_tagged!(
                serializer,
                recite_core::V0_DIVERT_TARGET_TAG_BLOCK,
                index.as_u32()
            ),
            CompiledDivertTarget::End => serialize_tagged!(
                serializer,
                recite_core::V0_DIVERT_TARGET_TAG_END,
                Option::<u8>::None
            ),
        }
    }
}

impl Serialize for MsgChoiceEcho<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.0 {
            CompiledChoiceEcho::None => serialize_tagged!(
                serializer,
                recite_core::V0_CHOICE_ECHO_TAG_NONE,
                Option::<u8>::None
            ),
            CompiledChoiceEcho::SelectedText => serialize_tagged!(
                serializer,
                recite_core::V0_CHOICE_ECHO_TAG_SELECTED_TEXT,
                Option::<u8>::None
            ),
            CompiledChoiceEcho::ExplicitLine(line_id) => serialize_tagged!(
                serializer,
                recite_core::V0_CHOICE_ECHO_TAG_EXPLICIT_LINE,
                line_id.as_str()
            ),
        }
    }
}

impl Serialize for MsgEffectMode {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.0 {
            CompiledEffectMode::Deferred => serialize_tagged!(
                serializer,
                recite_core::V0_EFFECT_MODE_TAG_DEFERRED,
                Option::<u8>::None
            ),
            CompiledEffectMode::Immediate => serialize_tagged!(
                serializer,
                recite_core::V0_EFFECT_MODE_TAG_IMMEDIATE,
                Option::<u8>::None
            ),
            CompiledEffectMode::Blocking => serialize_tagged!(
                serializer,
                recite_core::V0_EFFECT_MODE_TAG_BLOCKING,
                Option::<u8>::None
            ),
        }
    }
}

impl Serialize for MsgConditionExpression<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.0 {
            CompiledConditionExpression::Call(call) => serialize_tagged!(
                serializer,
                recite_core::V0_CONDITION_TAG_CALL,
                MsgConditionCall(call)
            ),
            CompiledConditionExpression::And(expressions) => serialize_tagged!(
                serializer,
                recite_core::V0_CONDITION_TAG_AND,
                expressions
                    .iter()
                    .map(MsgConditionExpression)
                    .collect::<Vec<_>>()
            ),
            CompiledConditionExpression::Or(expressions) => serialize_tagged!(
                serializer,
                recite_core::V0_CONDITION_TAG_OR,
                expressions
                    .iter()
                    .map(MsgConditionExpression)
                    .collect::<Vec<_>>()
            ),
            CompiledConditionExpression::Not(expression) => serialize_tagged!(
                serializer,
                recite_core::V0_CONDITION_TAG_NOT,
                MsgConditionExpression(expression)
            ),
        }
    }
}

impl Serialize for MsgConditionCall<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(2)?;
        tuple.serialize_element(self.0.function.as_str())?;
        tuple.serialize_element(&self.0.args.iter().map(MsgArgument).collect::<Vec<_>>())?;
        tuple.end()
    }
}

impl Serialize for MsgArgument<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.0 {
            CompiledArgument::Identifier(value) => {
                serialize_tagged!(serializer, recite_core::V0_ARGUMENT_TAG_IDENTIFIER, value)
            }
            CompiledArgument::Value(value) => serialize_tagged!(
                serializer,
                recite_core::V0_ARGUMENT_TAG_VALUE,
                MsgScalarValue(value)
            ),
        }
    }
}

impl Serialize for MsgValue<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.0 {
            Value::Scalar(value) => {
                serialize_tagged!(serializer, value_tag(self.0), MsgScalarValue(value))
            }
            Value::Array(values) => serialize_tagged!(
                serializer,
                value_tag(self.0),
                values.iter().map(MsgScalarValue).collect::<Vec<_>>()
            ),
        }
    }
}

impl Serialize for MsgScalarValue<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.0 {
            ScalarValue::String(value) => {
                serialize_tagged!(serializer, scalar_value_tag(self.0), value)
            }
            ScalarValue::Integer(value) => {
                serialize_tagged!(serializer, scalar_value_tag(self.0), value)
            }
            ScalarValue::Float(value) => {
                serialize_tagged!(serializer, scalar_value_tag(self.0), value)
            }
            ScalarValue::Boolean(value) => {
                serialize_tagged!(serializer, scalar_value_tag(self.0), value)
            }
        }
    }
}

impl Serialize for MsgSourceSpan<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(recite_core::V0_SOURCE_SPAN_FIELDS as usize)?;
        tuple.serialize_element(self.0.file.as_str())?;
        tuple.serialize_element(&self.0.start.line())?;
        tuple.serialize_element(&self.0.start.column())?;
        tuple.serialize_element(&self.0.end.map(|end| end.line()))?;
        tuple.serialize_element(&self.0.end.map(|end| end.column()))?;
        tuple.end()
    }
}
