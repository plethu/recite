use super::{
    range_to_u32,
    tags::{MsgArgument, MsgEffectMode, MsgSourceSpan, MsgValue},
};
use crate::{
    ChoiceIndex, MatchArmIndex, MetadataIndex, SourceMapIndex, StatementIndex, TableRange,
};
use serde::Serialize;
use serde::ser::SerializeTuple;

#[derive(Serialize)]
pub(super) struct MsgAvailabilityReason<'a>(pub(super) &'a str, pub(super) &'a str);

impl<'a> From<&'a crate::CompiledAvailabilityReason> for MsgAvailabilityReason<'a> {
    fn from(reason: &'a crate::CompiledAvailabilityReason) -> Self {
        Self(reason.id.as_str(), reason.template.as_str())
    }
}

#[derive(Serialize)]
pub(super) struct MsgConditionAvailabilityReason<'a>(
    &'a str,
    &'a str,
    Vec<MsgAvailabilityReasonArgBinding<'a>>,
);

impl<'a> From<&'a crate::CompiledConditionAvailabilityReason>
    for MsgConditionAvailabilityReason<'a>
{
    fn from(mapping: &'a crate::CompiledConditionAvailabilityReason) -> Self {
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
pub(super) struct MsgAvailabilityReasonArgBinding<'a>(
    pub(super) &'a str,
    pub(super) MsgAvailabilityReasonArgValue<'a>,
);

impl<'a> From<&'a crate::CompiledAvailabilityReasonArgBinding>
    for MsgAvailabilityReasonArgBinding<'a>
{
    fn from(binding: &'a crate::CompiledAvailabilityReasonArgBinding) -> Self {
        Self(
            binding.name.as_str(),
            MsgAvailabilityReasonArgValue(&binding.value),
        )
    }
}

pub(super) struct MsgAvailabilityReasonArgValue<'a>(
    pub(super) &'a crate::CompiledAvailabilityReasonArgValue,
);

impl Serialize for MsgAvailabilityReasonArgValue<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(crate::V0_TAGGED_VALUE_FIELDS as usize)?;
        match self.0 {
            crate::CompiledAvailabilityReasonArgValue::ConditionArg(value) => {
                tuple.serialize_element("ConditionArg")?;
                tuple.serialize_element(value)?;
            }
            crate::CompiledAvailabilityReasonArgValue::Literal(value) => match value {
                crate::ScalarValue::String(value) => {
                    tuple.serialize_element("LiteralString")?;
                    tuple.serialize_element(value)?;
                }
                crate::ScalarValue::Integer(value) => {
                    tuple.serialize_element("LiteralInt")?;
                    tuple.serialize_element(value)?;
                }
                crate::ScalarValue::Float(value) => {
                    tuple.serialize_element("LiteralFloat")?;
                    tuple.serialize_element(value)?;
                }
                crate::ScalarValue::Boolean(value) => {
                    tuple.serialize_element("LiteralBool")?;
                    tuple.serialize_element(value)?;
                }
            },
        }
        tuple.end()
    }
}

pub(super) struct MsgSpeaker<'a>(pub(super) &'a str);

impl<'a> From<&'a crate::CompiledSpeaker> for MsgSpeaker<'a> {
    fn from(speaker: &'a crate::CompiledSpeaker) -> Self {
        Self(speaker.id.as_str())
    }
}

impl Serialize for MsgSpeaker<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        let mut tuple = serializer.serialize_tuple(crate::V0_SPEAKER_FIELDS as usize)?;
        tuple.serialize_element(&self.0)?;
        tuple.end()
    }
}

#[derive(Serialize)]
pub(super) struct MsgMetadataEntry<'a>(
    pub(super) &'a str,
    pub(super) MsgValue<'a>,
    pub(super) Option<u32>,
);

impl<'a> From<&'a crate::CompiledMetadataEntry> for MsgMetadataEntry<'a> {
    fn from(entry: &'a crate::CompiledMetadataEntry) -> Self {
        Self(
            entry.key.as_str(),
            MsgValue(&entry.value),
            entry.source_map.map(SourceMapIndex::as_u32),
        )
    }
}

#[derive(Serialize)]
pub(super) struct MsgEffect<'a>(
    pub(super) &'a str,
    pub(super) MsgEffectMode,
    pub(super) &'a str,
    pub(super) Vec<MsgArgument<'a>>,
    pub(super) u32,
);

impl<'a> From<&'a crate::CompiledEffect> for MsgEffect<'a> {
    fn from(effect: &'a crate::CompiledEffect) -> Self {
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
pub(super) struct MsgSourceMapEntry<'a>(pub(super) u32, pub(super) MsgSourceSpan<'a>);

impl<'a> From<&'a crate::CompiledSourceMapEntry> for MsgSourceMapEntry<'a> {
    fn from(entry: &'a crate::CompiledSourceMapEntry) -> Self {
        Self(entry.source_file.as_u32(), MsgSourceSpan(&entry.span))
    }
}

#[derive(Serialize)]
pub(super) struct MsgLookupEntry<'a>(pub(super) &'a str, pub(super) u32);

#[derive(Serialize)]
pub(super) struct MsgRange(pub(super) u32, pub(super) u32);

pub(super) fn statement_range(range: TableRange<StatementIndex>) -> MsgRange {
    let (start, len) = range_to_u32(range, StatementIndex::as_u32);
    MsgRange(start, len)
}

pub(super) fn match_arm_range(range: TableRange<MatchArmIndex>) -> MsgRange {
    let (start, len) = range_to_u32(range, MatchArmIndex::as_u32);
    MsgRange(start, len)
}

pub(super) fn choice_range(range: TableRange<ChoiceIndex>) -> MsgRange {
    let (start, len) = range_to_u32(range, ChoiceIndex::as_u32);
    MsgRange(start, len)
}

pub(super) fn metadata_range(range: TableRange<MetadataIndex>) -> MsgRange {
    let (start, len) = range_to_u32(range, MetadataIndex::as_u32);
    MsgRange(start, len)
}
