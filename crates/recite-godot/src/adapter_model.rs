use recite_core::{MetadataEntry, SourceSpan, Value};
use recite_runtime::{
    ChoiceAvailability, ChoiceAvailabilityReason, ChoiceAvailabilityReasonArg,
    ChoiceAvailabilityReasonOrigin, ChoiceAvailabilityReasonTree, ChoiceAvailabilityReasonValue,
    DialogueEffectArgument, DialogueEffectMode,
};

use crate::adapter::AdapterValue;

pub(crate) fn source_span_parts(span: &SourceSpan) -> (&str, u32, u32, Option<(u32, u32)>) {
    (
        &span.file,
        span.start.line(),
        span.start.column(),
        span.end.map(|end| (end.line(), end.column())),
    )
}

pub(crate) fn metadata_entries(entries: &[MetadataEntry]) -> Vec<(&str, &Value)> {
    entries
        .iter()
        .map(|entry| (entry.key.as_str(), &entry.value))
        .collect()
}

pub(crate) fn effect_mode_name(mode: DialogueEffectMode) -> &'static str {
    match mode {
        DialogueEffectMode::Deferred => "deferred",
        DialogueEffectMode::Immediate => "immediate",
        DialogueEffectMode::Blocking => "blocking",
    }
}

pub(crate) fn effect_arg_value(argument: &DialogueEffectArgument) -> AdapterValue {
    match argument {
        DialogueEffectArgument::Identifier(value) => AdapterValue::Identifier(value.clone()),
        DialogueEffectArgument::String(value) => AdapterValue::String(value.clone()),
        DialogueEffectArgument::Integer(value) => AdapterValue::Integer(*value),
        DialogueEffectArgument::Float(value) => AdapterValue::Float(*value),
        DialogueEffectArgument::Boolean(value) => AdapterValue::Boolean(*value),
    }
}

pub(crate) fn reason_value_parts(value: &ChoiceAvailabilityReasonValue) -> AdapterValue {
    match value {
        ChoiceAvailabilityReasonValue::Identifier(value) => AdapterValue::Identifier(value.clone()),
        ChoiceAvailabilityReasonValue::String(value) => AdapterValue::String(value.clone()),
        ChoiceAvailabilityReasonValue::Integer(value) => AdapterValue::Integer(*value),
        ChoiceAvailabilityReasonValue::Float(value) => AdapterValue::Float(*value),
        ChoiceAvailabilityReasonValue::Boolean(value) => AdapterValue::Boolean(*value),
    }
}

pub(crate) fn reason_origin_kind(origin: &ChoiceAvailabilityReasonOrigin) -> &'static str {
    match origin {
        ChoiceAvailabilityReasonOrigin::ConditionCall { .. } => "condition_call",
        ChoiceAvailabilityReasonOrigin::RequirementExpression { .. } => "requirement_expression",
    }
}

pub(crate) fn availability_summary(availability: &ChoiceAvailability) -> (bool, bool, bool) {
    (
        availability.is_available,
        availability.primary_reason.is_some(),
        availability.reason_tree.is_some(),
    )
}

pub(crate) fn reason_tree_kind(tree: &ChoiceAvailabilityReasonTree) -> &'static str {
    match tree {
        ChoiceAvailabilityReasonTree::All(_) => "all",
        ChoiceAvailabilityReasonTree::Any(_) => "any",
        ChoiceAvailabilityReasonTree::Reason(_) => "reason",
        ChoiceAvailabilityReasonTree::RequirementSourceText(_) => "requirement_source_text",
    }
}

pub(crate) fn reason_arg_parts(arg: &ChoiceAvailabilityReasonArg) -> (&str, AdapterValue) {
    (arg.name.as_str(), reason_value_parts(&arg.value))
}

pub(crate) fn reason_id_text(reason: &ChoiceAvailabilityReason) -> (&str, &str, &str) {
    (
        reason.id.as_str(),
        reason.source_text.as_str(),
        reason.text.as_str(),
    )
}
