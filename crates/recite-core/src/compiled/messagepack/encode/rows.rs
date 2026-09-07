use crate::{
    CompiledChoice, CompiledInterpolationBinding, CompiledInterpolationMode, CompiledLine,
    SpeakerIndex,
};
use serde::{Serialize, Serializer};

use super::tables::MsgRange;
use super::tags::{MsgChoiceEcho, MsgConditionExpression, MsgDivertTarget};

#[derive(Serialize)]
pub(super) struct MsgLineCurrent<'a>(
    &'a str,
    &'a str,
    Option<u32>,
    MsgRange,
    u32,
    &'a str,
    Vec<MsgInterpolationBinding<'a>>,
    Option<&'a str>,
    Option<&'a str>,
);

#[derive(Serialize)]
pub(super) struct MsgLineLegacy<'a>(&'a str, &'a str, Option<u32>, MsgRange, u32);

pub(super) enum MsgLine<'a> {
    Current(MsgLineCurrent<'a>),
    Legacy(MsgLineLegacy<'a>),
}

impl Serialize for MsgLine<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Current(line) => line.serialize(serializer),
            Self::Legacy(line) => line.serialize(serializer),
        }
    }
}

impl<'a> From<&'a CompiledLine> for MsgLine<'a> {
    fn from(line: &'a CompiledLine) -> Self {
        match line.interpolation_mode {
            CompiledInterpolationMode::Current => Self::Current(MsgLineCurrent(
                line.id.as_str(),
                line.source_text.as_str(),
                line.speaker.map(SpeakerIndex::as_u32),
                super::tables::metadata_range(line.metadata),
                line.source_map.as_u32(),
                line.authored_source_text.as_str(),
                line.interpolation_bindings
                    .iter()
                    .map(MsgInterpolationBinding::from)
                    .collect(),
                line.plural_source_text.as_deref(),
                line.authored_plural_source_text.as_deref(),
            )),
            CompiledInterpolationMode::Legacy => Self::Legacy(MsgLineLegacy(
                line.id.as_str(),
                line.source_text.as_str(),
                line.speaker.map(SpeakerIndex::as_u32),
                super::tables::metadata_range(line.metadata),
                line.source_map.as_u32(),
            )),
        }
    }
}

#[derive(Serialize)]
pub(super) struct MsgChoiceCurrent<'a>(
    &'a str,
    &'a str,
    MsgRange,
    Option<MsgConditionExpression<'a>>,
    Option<&'a str>,
    Option<&'a str>,
    MsgDivertTarget<'a>,
    MsgChoiceEcho<'a>,
    u32,
    &'a str,
    Vec<MsgInterpolationBinding<'a>>,
);

#[derive(Serialize)]
pub(super) struct MsgChoiceLegacy<'a>(
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

pub(super) enum MsgChoice<'a> {
    Current(MsgChoiceCurrent<'a>),
    Legacy(MsgChoiceLegacy<'a>),
}

impl Serialize for MsgChoice<'_> {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: Serializer,
    {
        match self {
            Self::Current(choice) => choice.serialize(serializer),
            Self::Legacy(choice) => choice.serialize(serializer),
        }
    }
}

impl<'a> From<&'a CompiledChoice> for MsgChoice<'a> {
    fn from(choice: &'a CompiledChoice) -> Self {
        let current = || {
            MsgChoiceCurrent(
                choice.id.as_str(),
                choice.source_text.as_str(),
                super::tables::metadata_range(choice.metadata),
                choice
                    .availability_requirement
                    .as_ref()
                    .map(MsgConditionExpression),
                choice.availability_requirement_source_text.as_deref(),
                choice
                    .availability_reason_override
                    .as_ref()
                    .map(crate::AvailabilityReasonId::as_str),
                MsgDivertTarget(&choice.target),
                MsgChoiceEcho(&choice.echo),
                choice.source_map.as_u32(),
                choice.authored_source_text.as_str(),
                choice
                    .interpolation_bindings
                    .iter()
                    .map(MsgInterpolationBinding::from)
                    .collect(),
            )
        };
        match choice.interpolation_mode {
            CompiledInterpolationMode::Current => Self::Current(current()),
            CompiledInterpolationMode::Legacy => Self::Legacy(MsgChoiceLegacy(
                choice.id.as_str(),
                choice.source_text.as_str(),
                super::tables::metadata_range(choice.metadata),
                choice
                    .availability_requirement
                    .as_ref()
                    .map(MsgConditionExpression),
                choice.availability_requirement_source_text.as_deref(),
                choice
                    .availability_reason_override
                    .as_ref()
                    .map(crate::AvailabilityReasonId::as_str),
                MsgDivertTarget(&choice.target),
                MsgChoiceEcho(&choice.echo),
                choice.source_map.as_u32(),
            )),
        }
    }
}

#[derive(Serialize)]
pub(super) struct MsgInterpolationBinding<'a>(&'a str, &'a str, &'static str);

impl<'a> From<&'a CompiledInterpolationBinding> for MsgInterpolationBinding<'a> {
    fn from(binding: &'a CompiledInterpolationBinding) -> Self {
        let value_type = match binding.value_type {
            crate::InterpolationType::String => "string",
            crate::InterpolationType::Integer => "int",
            crate::InterpolationType::Float => "float",
            crate::InterpolationType::Boolean => "bool",
        };
        Self(&binding.name, &binding.value, value_type)
    }
}
