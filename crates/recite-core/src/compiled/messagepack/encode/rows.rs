use crate::{CompiledChoice, CompiledInterpolationBinding, CompiledLine, SpeakerIndex};
use serde::Serialize;

use super::tables::MsgRange;
use super::tags::{MsgChoiceEcho, MsgConditionExpression, MsgDivertTarget};

#[derive(Serialize)]
pub(super) struct MsgLine<'a>(
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

impl<'a> From<&'a CompiledLine> for MsgLine<'a> {
    fn from(line: &'a CompiledLine) -> Self {
        Self(
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
        )
    }
}

#[derive(Serialize)]
pub(super) struct MsgChoice<'a>(
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

impl<'a> From<&'a CompiledChoice> for MsgChoice<'a> {
    fn from(choice: &'a CompiledChoice) -> Self {
        Self(
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
