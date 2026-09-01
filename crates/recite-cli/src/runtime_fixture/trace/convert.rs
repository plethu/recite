use recite_core::{MetadataEntry, ScalarValue, SourceSpan, Value};
use recite_runtime::{
    DialogueChoice, DialogueEffectArgument, DialogueEffectRequest, DialogueLine, PreviewTrace,
};

use super::availability::trace_availability;
use super::format::effect_mode_name;
use super::model::{
    TraceChoice, TraceEffect, TraceLine, TraceMetadata, TracePlural, TracePluralAttempt,
    TraceScalar, TraceSourceSpan, TraceValue,
};

pub(in crate::runtime_fixture) fn trace_line(
    line: &DialogueLine,
    dialogue_trace: &PreviewTrace,
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
            .map(|trace| trace_plural(trace.clone())),
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
    dialogue_trace: &PreviewTrace,
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
