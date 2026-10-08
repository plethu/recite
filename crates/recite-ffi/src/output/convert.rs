use recite_core::{ScalarValue, Value};
use recite_runtime::{
    ChoiceAvailability, ChoiceAvailabilityReason, ChoiceAvailabilityReasonOrigin,
    ChoiceAvailabilityReasonTree, ChoiceEchoMode, DialogueEffectMode, DialogueEffectRequest,
    DialogueEvent, DialogueLine, DialoguePlural, DialoguePluralResolutionOutcome,
    localisation::{PluralResolutionAttempt, PluralResolutionOutcome},
};

use super::model::{
    FfiAvailability, FfiAvailabilityReason, FfiChoice, FfiEcho, FfiEffect, FfiEvent, FfiLine,
    FfiMetaValue, FfiMetadata, FfiPlural, FfiPluralAttempt, FfiPluralResolution, FfiReasonArg,
    FfiReasonOrigin, FfiReasonTree, FfiScalar,
};
use crate::tagged_value::TaggedValue;

pub(crate) fn ffi_event(event: DialogueEvent) -> FfiEvent {
    match event {
        DialogueEvent::Line(line) => FfiEvent::Line(ffi_line(line)),
        DialogueEvent::Prompt { line, choices } => FfiEvent::Prompt {
            line: line.map(ffi_line),
            choices: choices.into_iter().map(ffi_choice).collect(),
        },
        DialogueEvent::Effect(effect) => FfiEvent::Effect(ffi_effect(effect)),
        DialogueEvent::End { deferred_effects } => FfiEvent::End {
            deferred_effects: deferred_effects.into_iter().map(ffi_effect).collect(),
        },
    }
}

fn ffi_line(line: DialogueLine) -> FfiLine {
    FfiLine {
        id: line.id.as_str().to_owned(),
        source_text: line.source_text,
        text: line.text,
        speaker: line.speaker.map(|s| s.as_str().to_owned()),
        metadata: line.metadata.into_iter().map(ffi_metadata).collect(),
        plural: line.plural.map(ffi_plural),
    }
}

fn ffi_plural(plural: DialoguePlural) -> FfiPlural {
    FfiPlural {
        singular_source_text: plural.singular_source_text,
        plural_source_text: plural.plural_source_text,
        count: plural.count,
        selected_arm: plural.selected_arm,
        resolution: FfiPluralResolution {
            attempts: plural
                .resolution
                .attempts
                .into_iter()
                .map(ffi_plural_attempt)
                .collect(),
            matched_locale: plural.resolution.matched_locale,
            matched_context: plural.resolution.matched_context,
            matched_key: plural.resolution.matched_key,
            matched_arm: plural.resolution.matched_arm,
            source_fallback_arm: plural.resolution.source_fallback_arm,
            outcome: match plural.resolution.outcome {
                DialoguePluralResolutionOutcome::Translated => "translated",
                DialoguePluralResolutionOutcome::EnglishSourceFallback => "english_source_fallback",
            },
        },
    }
}

fn ffi_plural_attempt(attempt: PluralResolutionAttempt) -> FfiPluralAttempt {
    FfiPluralAttempt {
        locale: attempt.locale,
        context: attempt.context,
        key: attempt.key,
        selected_arm: attempt.selected_arm,
        outcome: plural_outcome_name(attempt.outcome),
    }
}

fn plural_outcome_name(outcome: PluralResolutionOutcome) -> &'static str {
    match outcome {
        PluralResolutionOutcome::MissingPluralForms => "missing_plural_forms",
        PluralResolutionOutcome::MissingEntry => "missing_entry",
        PluralResolutionOutcome::MissingTranslation => "missing_translation",
        PluralResolutionOutcome::Matched => "matched",
    }
}

fn ffi_choice(choice: recite_runtime::DialogueChoice) -> FfiChoice {
    FfiChoice {
        id: choice.id.as_str().to_owned(),
        source_text: choice.source_text,
        text: choice.text,
        metadata: choice.metadata.into_iter().map(ffi_metadata).collect(),
        echo: ffi_echo(choice.echo),
        availability: ffi_availability(choice.availability),
    }
}

fn ffi_echo(echo: ChoiceEchoMode) -> FfiEcho {
    match echo {
        ChoiceEchoMode::None => FfiEcho {
            kind: "none",
            explicit_line_id: None,
        },
        ChoiceEchoMode::SelectedText => FfiEcho {
            kind: "selected_text",
            explicit_line_id: None,
        },
        ChoiceEchoMode::ExplicitLine(id) => FfiEcho {
            kind: "explicit_line",
            explicit_line_id: Some(id.as_str().to_owned()),
        },
    }
}

fn ffi_availability(av: ChoiceAvailability) -> FfiAvailability {
    FfiAvailability {
        is_available: av.is_available,
        primary_reason: av.primary_reason.map(ffi_availability_reason),
        reason_tree: av.reason_tree.map(ffi_reason_tree),
    }
}

fn ffi_availability_reason(reason: ChoiceAvailabilityReason) -> FfiAvailabilityReason {
    FfiAvailabilityReason {
        id: reason.id.as_str().to_owned(),
        source_text: reason.source_text,
        text: reason.text,
        origin: reason.origin.map(ffi_reason_origin),
        args: reason
            .args
            .into_iter()
            .map(|arg| FfiReasonArg {
                name: arg.name,
                value: arg.value.into(),
            })
            .collect(),
    }
}

fn ffi_reason_tree(tree: ChoiceAvailabilityReasonTree) -> FfiReasonTree {
    match tree {
        ChoiceAvailabilityReasonTree::All(children) => FfiReasonTree::All {
            children: children.into_iter().map(ffi_reason_tree).collect(),
        },
        ChoiceAvailabilityReasonTree::Any(children) => FfiReasonTree::Any {
            children: children.into_iter().map(ffi_reason_tree).collect(),
        },
        ChoiceAvailabilityReasonTree::Reason(reason) => {
            FfiReasonTree::Reason(ffi_availability_reason(reason))
        }
        ChoiceAvailabilityReasonTree::RequirementSourceText(text) => {
            FfiReasonTree::RequirementSourceText { text }
        }
    }
}

fn ffi_reason_origin(origin: ChoiceAvailabilityReasonOrigin) -> FfiReasonOrigin {
    match origin {
        ChoiceAvailabilityReasonOrigin::ConditionCall { function, args } => {
            FfiReasonOrigin::ConditionCall {
                function,
                args: args.into_iter().map(TaggedValue::from).collect(),
            }
        }
        ChoiceAvailabilityReasonOrigin::RequirementExpression { source_text } => {
            FfiReasonOrigin::RequirementExpression { source_text }
        }
    }
}

fn ffi_effect(effect: DialogueEffectRequest) -> FfiEffect {
    let mode = match effect.mode {
        DialogueEffectMode::Deferred => "deferred",
        DialogueEffectMode::Immediate => "immediate",
        DialogueEffectMode::Blocking => "blocking",
    };
    FfiEffect {
        id: effect.id.as_str().to_owned(),
        mode,
        function: effect.function,
        args: effect.args.into_iter().map(TaggedValue::from).collect(),
        source_file: effect.source_span.file,
        source_line: effect.source_span.start.line(),
        source_col: effect.source_span.start.column(),
    }
}

fn ffi_metadata(entry: recite_core::MetadataEntry) -> FfiMetadata {
    FfiMetadata {
        key: entry.key,
        value: ffi_meta_value(entry.value),
    }
}

fn ffi_meta_value(value: Value) -> FfiMetaValue {
    match value {
        Value::Scalar(scalar) => ffi_scalar_as_meta(scalar),
        Value::Array(items) => FfiMetaValue::Array {
            values: items.into_iter().map(ffi_scalar).collect(),
        },
    }
}

fn ffi_scalar_as_meta(scalar: ScalarValue) -> FfiMetaValue {
    match scalar {
        ScalarValue::String(v) => FfiMetaValue::String { value: v },
        ScalarValue::Integer(v) => FfiMetaValue::Integer { value: v },
        ScalarValue::Float(v) => FfiMetaValue::Float { value: v },
        ScalarValue::Boolean(v) => FfiMetaValue::Boolean { value: v },
    }
}

fn ffi_scalar(scalar: ScalarValue) -> FfiScalar {
    match scalar {
        ScalarValue::String(v) => FfiScalar::String { value: v },
        ScalarValue::Integer(v) => FfiScalar::Integer { value: v },
        ScalarValue::Float(v) => FfiScalar::Float { value: v },
        ScalarValue::Boolean(v) => FfiScalar::Boolean { value: v },
    }
}
