use super::super::tables::{choice_range, match_arm_range, statement_range};
use super::{
    MsgAssetEncoding, MsgChoiceEcho, MsgConditionCall, MsgConditionExpression, MsgDivertTarget,
    MsgEffectMode, MsgFingerprint, MsgInspectionEncoding, MsgMatchPattern, MsgSchemaFingerprint,
    MsgStatementKind,
};
use crate::{
    CompiledAssetEncoding, CompiledChoiceEcho, CompiledDivertTarget, CompiledEffectMode,
    CompiledInspectionEncoding, CompiledMatchPattern, CompiledStatementKind, LineIndex,
    SchemaFingerprint,
};
use serde::Serialize;
use serde::ser::SerializeTuple;

impl Serialize for MsgAssetEncoding {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        match self.0 {
            CompiledAssetEncoding::MessagePack => serialize_tagged!(
                serializer,
                crate::V0_ASSET_ENCODING_MESSAGEPACK,
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
                crate::V0_INSPECTION_ENCODING_COMPACT_JSON,
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
                crate::V0_SCHEMA_FINGERPRINT_TAG_FINGERPRINT,
                MsgFingerprint(fingerprint)
            ),
            SchemaFingerprint::NoSchema => serialize_tagged!(
                serializer,
                crate::V0_SCHEMA_FINGERPRINT_TAG_NO_SCHEMA,
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
        let mut tuple = serializer.serialize_tuple(crate::V0_FINGERPRINT_FIELDS as usize)?;
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
                serialize_tagged!(serializer, crate::V0_STATEMENT_TAG_LINE, index.as_u32())
            }
            CompiledStatementKind::Prompt { line, choices } => serialize_tagged!(
                serializer,
                crate::V0_STATEMENT_TAG_PROMPT,
                (line.map(LineIndex::as_u32), choice_range(*choices))
            ),
            CompiledStatementKind::Divert(target) => serialize_tagged!(
                serializer,
                crate::V0_STATEMENT_TAG_DIVERT,
                MsgDivertTarget(target)
            ),
            CompiledStatementKind::If {
                condition,
                then_statements,
                else_statements,
            } => serialize_tagged!(
                serializer,
                crate::V0_STATEMENT_TAG_IF,
                (
                    MsgConditionExpression(condition),
                    statement_range(*then_statements),
                    statement_range(*else_statements)
                )
            ),
            CompiledStatementKind::Match { scrutinee, arms } => serialize_tagged!(
                serializer,
                crate::V0_STATEMENT_TAG_MATCH,
                (MsgConditionCall(scrutinee), match_arm_range(*arms))
            ),
            CompiledStatementKind::Effect(index) => {
                serialize_tagged!(serializer, crate::V0_STATEMENT_TAG_EFFECT, index.as_u32())
            }
            CompiledStatementKind::End => {
                serialize_tagged!(serializer, crate::V0_STATEMENT_TAG_END, Option::<u8>::None)
            }
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
                serialize_tagged!(serializer, crate::V0_MATCH_PATTERN_TAG_VARIANT, value)
            }
            CompiledMatchPattern::Wildcard => serialize_tagged!(
                serializer,
                crate::V0_MATCH_PATTERN_TAG_WILDCARD,
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
                crate::V0_DIVERT_TARGET_TAG_BLOCK,
                index.as_u32()
            ),
            CompiledDivertTarget::End => serialize_tagged!(
                serializer,
                crate::V0_DIVERT_TARGET_TAG_END,
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
                crate::V0_CHOICE_ECHO_TAG_NONE,
                Option::<u8>::None
            ),
            CompiledChoiceEcho::SelectedText => serialize_tagged!(
                serializer,
                crate::V0_CHOICE_ECHO_TAG_SELECTED_TEXT,
                Option::<u8>::None
            ),
            CompiledChoiceEcho::ExplicitLine(line_id) => serialize_tagged!(
                serializer,
                crate::V0_CHOICE_ECHO_TAG_EXPLICIT_LINE,
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
                crate::V0_EFFECT_MODE_TAG_DEFERRED,
                Option::<u8>::None
            ),
            CompiledEffectMode::Immediate => serialize_tagged!(
                serializer,
                crate::V0_EFFECT_MODE_TAG_IMMEDIATE,
                Option::<u8>::None
            ),
            CompiledEffectMode::Blocking => serialize_tagged!(
                serializer,
                crate::V0_EFFECT_MODE_TAG_BLOCKING,
                Option::<u8>::None
            ),
        }
    }
}
