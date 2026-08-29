use recite_core::{MetadataEntry, ScalarValue, SourceSpan, Value};
use recite_runtime::{
    ChoiceAvailability, ChoiceAvailabilityReason, ChoiceAvailabilityReasonArg,
    ChoiceAvailabilityReasonOrigin, ChoiceAvailabilityReasonTree, ChoiceAvailabilityReasonValue,
    ConditionArgument, DialogueChoice, DialogueEffectArgument, DialogueEffectRequest, DialogueLine,
    DialogueTrace,
};

use super::format::effect_mode_name;
use super::model::{
    TraceChoice, TraceChoiceAvailability, TraceChoiceAvailabilityReason,
    TraceChoiceAvailabilityReasonArg, TraceChoiceAvailabilityReasonOrigin,
    TraceChoiceAvailabilityReasonTree, TraceChoiceAvailabilityReasonValue, TraceEffect, TraceLine,
    TraceMetadata, TracePlural, TracePluralAttempt, TraceScalar, TraceSourceSpan, TraceValue,
};

pub(in crate::runtime_fixture) fn trace_line(
    line: &DialogueLine,
    dialogue_trace: &DialogueTrace,
) -> TraceLine {
    TraceLine {
        id: line.id.as_str().to_owned(),
        source_text: line.source_text.clone(),
        text: line.text.clone(),
        speaker: line
            .speaker
            .as_ref()
            .map(|speaker| speaker.as_str().to_owned()),
        metadata: line.metadata.iter().map(trace_metadata).collect(),
        plural: dialogue_trace
            .plural_line(line.id.as_str())
            .map(trace_plural),
    }
}

fn trace_plural(trace: recite_runtime::PluralLineTrace) -> TracePlural {
    TracePlural {
        singular_source_text: trace.singular_source_text,
        plural_source_text: trace.plural_source_text,
        count: trace.count,
        selected_arm: trace.selected_arm,
        attempts: trace
            .attempts
            .into_iter()
            .map(|attempt| TracePluralAttempt {
                locale: attempt.locale,
                context: attempt.context,
                key: attempt.key,
                selected_arm: attempt.selected_arm,
                outcome: match attempt.outcome {
                    recite_runtime::PluralResolutionOutcome::MissingPluralForms => {
                        "missing_plural_forms"
                    }
                    recite_runtime::PluralResolutionOutcome::MissingEntry => "missing_entry",
                    recite_runtime::PluralResolutionOutcome::MissingTranslation => {
                        "missing_translation"
                    }
                    recite_runtime::PluralResolutionOutcome::Matched => "matched",
                },
            })
            .collect(),
        matched_locale: trace.matched_locale,
        matched_context: trace.matched_context,
        matched_key: trace.matched_key,
        matched_arm: trace.matched_arm,
        source_fallback_arm: trace.source_fallback_arm,
        outcome: if trace.source_fallback_arm.is_some() {
            "english_source_fallback"
        } else {
            "translated"
        },
    }
}

pub(in crate::runtime_fixture) fn trace_choice(
    choice: &DialogueChoice,
    dialogue_trace: &DialogueTrace,
) -> TraceChoice {
    TraceChoice {
        id: choice.id.as_str().to_owned(),
        source_text: choice.source_text.clone(),
        text: choice.text.clone(),
        metadata: choice.metadata.iter().map(trace_metadata).collect(),
        is_available: choice.availability.is_available,
        availability: trace_availability(&choice.availability, dialogue_trace),
        unavailable_reason: choice
            .availability
            .primary_reason
            .as_ref()
            .map(|reason| reason.text.clone()),
    }
}

fn trace_availability(
    availability: &ChoiceAvailability,
    dialogue_trace: &DialogueTrace,
) -> TraceChoiceAvailability {
    TraceChoiceAvailability {
        is_available: availability.is_available,
        primary_reason: availability
            .primary_reason
            .as_ref()
            .map(|reason| trace_availability_reason(reason, dialogue_trace)),
        reason_tree: availability
            .reason_tree
            .as_ref()
            .map(|tree| trace_availability_reason_tree(tree, dialogue_trace)),
    }
}

fn trace_availability_reason(
    reason: &ChoiceAvailabilityReason,
    dialogue_trace: &DialogueTrace,
) -> TraceChoiceAvailabilityReason {
    TraceChoiceAvailabilityReason {
        id: reason.id.as_str().to_owned(),
        source_text: reason.source_text.clone(),
        localized_template: dialogue_trace
            .localized_availability_template(reason.id.as_str())
            .unwrap_or_else(|| reason.source_text.clone()),
        text: reason.text.clone(),
        origin: reason.origin.as_ref().map(trace_availability_reason_origin),
        args: reason
            .args
            .iter()
            .map(trace_availability_reason_arg)
            .collect(),
    }
}

fn trace_availability_reason_origin(
    origin: &ChoiceAvailabilityReasonOrigin,
) -> TraceChoiceAvailabilityReasonOrigin {
    match origin {
        ChoiceAvailabilityReasonOrigin::ConditionCall { function, args } => {
            TraceChoiceAvailabilityReasonOrigin::ConditionCall {
                function: function.clone(),
                args: args.iter().map(trace_availability_reason_value).collect(),
            }
        }
        ChoiceAvailabilityReasonOrigin::RequirementExpression { source_text } => {
            TraceChoiceAvailabilityReasonOrigin::RequirementExpression {
                source_text: source_text.clone(),
            }
        }
    }
}

fn trace_availability_reason_arg(
    arg: &ChoiceAvailabilityReasonArg,
) -> TraceChoiceAvailabilityReasonArg {
    TraceChoiceAvailabilityReasonArg {
        name: arg.name.clone(),
        value: trace_availability_reason_value(&arg.value),
    }
}

fn trace_availability_reason_value(
    value: &ChoiceAvailabilityReasonValue,
) -> TraceChoiceAvailabilityReasonValue {
    match value {
        ChoiceAvailabilityReasonValue::Identifier(value) => {
            TraceChoiceAvailabilityReasonValue::Identifier(value.clone())
        }
        ChoiceAvailabilityReasonValue::String(value) => {
            TraceChoiceAvailabilityReasonValue::String(value.clone())
        }
        ChoiceAvailabilityReasonValue::Integer(value) => {
            TraceChoiceAvailabilityReasonValue::Integer(*value)
        }
        ChoiceAvailabilityReasonValue::Float(value) => {
            TraceChoiceAvailabilityReasonValue::Float(*value)
        }
        ChoiceAvailabilityReasonValue::Boolean(value) => {
            TraceChoiceAvailabilityReasonValue::Boolean(*value)
        }
    }
}

fn trace_availability_reason_tree(
    tree: &ChoiceAvailabilityReasonTree,
    dialogue_trace: &DialogueTrace,
) -> TraceChoiceAvailabilityReasonTree {
    match tree {
        ChoiceAvailabilityReasonTree::All(children) => TraceChoiceAvailabilityReasonTree::All(
            children
                .iter()
                .map(|tree| trace_availability_reason_tree(tree, dialogue_trace))
                .collect(),
        ),
        ChoiceAvailabilityReasonTree::Any(children) => TraceChoiceAvailabilityReasonTree::Any(
            children
                .iter()
                .map(|tree| trace_availability_reason_tree(tree, dialogue_trace))
                .collect(),
        ),
        ChoiceAvailabilityReasonTree::Reason(reason) => TraceChoiceAvailabilityReasonTree::Reason(
            trace_availability_reason(reason, dialogue_trace),
        ),
        ChoiceAvailabilityReasonTree::RequirementSourceText(source_text) => {
            TraceChoiceAvailabilityReasonTree::RequirementSourceText(source_text.clone())
        }
    }
}

fn trace_metadata(metadata: &MetadataEntry) -> TraceMetadata {
    TraceMetadata {
        key: metadata.key.clone(),
        value: trace_value(&metadata.value),
    }
}

fn trace_value(value: &Value) -> TraceValue {
    match value {
        Value::Scalar(value) => TraceValue::Scalar(trace_scalar(value)),
        Value::Array(values) => TraceValue::Array(values.iter().map(trace_scalar).collect()),
    }
}

fn trace_scalar(value: &ScalarValue) -> TraceScalar {
    match value {
        ScalarValue::String(value) => TraceScalar::String(value.clone()),
        ScalarValue::Integer(value) => TraceScalar::Integer(*value),
        ScalarValue::Float(value) => TraceScalar::Float(*value),
        ScalarValue::Boolean(value) => TraceScalar::Boolean(*value),
    }
}

pub(in crate::runtime_fixture) fn trace_effect(effect: &DialogueEffectRequest) -> TraceEffect {
    TraceEffect {
        id: effect.id.as_str().to_owned(),
        mode: effect_mode_name(effect.mode),
        function: effect.function.clone(),
        args: effect.args.iter().map(trace_effect_argument).collect(),
        source_span: trace_source_span(&effect.source_span),
    }
}

fn trace_source_span(span: &SourceSpan) -> TraceSourceSpan {
    TraceSourceSpan {
        file: span.file.clone(),
        start_line: span.start.line(),
        start_column: span.start.column(),
        end_line: span.end.map(|end| end.line()),
        end_column: span.end.map(|end| end.column()),
    }
}

pub(in crate::runtime_fixture) fn trace_condition_argument(
    argument: ConditionArgument<'_>,
) -> TraceScalar {
    match argument {
        ConditionArgument::Identifier(value) => TraceScalar::Identifier(value.to_owned()),
        ConditionArgument::String(value) => TraceScalar::String(value.to_owned()),
        ConditionArgument::Integer(value) => TraceScalar::Integer(value),
        ConditionArgument::Float(value) => TraceScalar::Float(value),
        ConditionArgument::Boolean(value) => TraceScalar::Boolean(value),
    }
}

pub(in crate::runtime_fixture) fn trace_effect_argument(
    argument: &DialogueEffectArgument,
) -> TraceScalar {
    match argument {
        DialogueEffectArgument::Identifier(value) => TraceScalar::Identifier(value.clone()),
        DialogueEffectArgument::String(value) => TraceScalar::String(value.clone()),
        DialogueEffectArgument::Integer(value) => TraceScalar::Integer(*value),
        DialogueEffectArgument::Float(value) => TraceScalar::Float(*value),
        DialogueEffectArgument::Boolean(value) => TraceScalar::Boolean(*value),
    }
}
