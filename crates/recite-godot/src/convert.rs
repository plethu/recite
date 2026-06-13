use godot::builtin::{PackedByteArray, VarArray, VarDictionary, Variant};
use godot::prelude::ToGodot;
use recite_core::{ScalarValue, Value};
use recite_runtime::{
    ChoiceAvailability, ChoiceAvailabilityReason, ChoiceAvailabilityReasonOrigin,
    ChoiceAvailabilityReasonTree, DialogueChoice, DialogueEffectRequest, DialogueLine,
};

use crate::adapter::{AdapterError, AdapterValue, ReciteOutput};
use crate::adapter_model::{
    availability_summary, choice_echo_name, effect_arg_value, effect_mode_name, metadata_entries,
    reason_arg_parts, reason_id_text, reason_origin_kind, reason_tree_kind, reason_value_parts,
    scalar_value_kind, source_span_parts,
};

pub(crate) fn result_dictionary(
    ok: bool,
    outputs: &VarArray,
    snapshot_bytes: &PackedByteArray,
    error: &VarDictionary,
) -> VarDictionary {
    let mut dictionary = VarDictionary::new();
    dictionary.set("ok", ok);
    set_variant(&mut dictionary, "outputs", outputs.to_variant());
    set_variant(
        &mut dictionary,
        "snapshot_bytes",
        snapshot_bytes.to_variant(),
    );
    set_variant(&mut dictionary, "error", error.to_variant());
    dictionary
}

pub(crate) fn outputs_array(outputs: &[ReciteOutput]) -> VarArray {
    let mut array = VarArray::new();
    for output in outputs {
        push_variant(&mut array, output_dictionary(output).to_variant());
    }
    array
}

pub(crate) fn output_dictionary(output: &ReciteOutput) -> VarDictionary {
    let mut dictionary = VarDictionary::new();
    match output {
        ReciteOutput::Line(line) => {
            dictionary.set("kind", "line");
            set_variant(&mut dictionary, "line", line_dictionary(line).to_variant());
        }
        ReciteOutput::Prompt { line, choices } => {
            dictionary.set("kind", "prompt");
            set_variant(
                &mut dictionary,
                "line",
                line.as_ref()
                    .map_or_else(Variant::nil, |line| line_dictionary(line).to_variant()),
            );
            set_variant(
                &mut dictionary,
                "choices",
                choices_array(choices).to_variant(),
            );
        }
        ReciteOutput::Effect(effect) => {
            dictionary.set("kind", "effect");
            set_variant(
                &mut dictionary,
                "effect",
                effect_dictionary(effect).to_variant(),
            );
        }
        ReciteOutput::End { deferred_effects } => {
            dictionary.set("kind", "end");
            set_variant(
                &mut dictionary,
                "deferred_effects",
                effects_array(deferred_effects).to_variant(),
            );
        }
    }
    dictionary
}

pub(crate) fn error_dictionary(error: &AdapterError) -> VarDictionary {
    let mut dictionary = VarDictionary::new();
    dictionary.set("kind", format!("{:?}", error.kind()));
    dictionary.set("code", error.code());
    dictionary.set("message", error.message());
    dictionary
}

pub(crate) fn bytes_to_packed(bytes: &[u8]) -> PackedByteArray {
    PackedByteArray::from(bytes)
}

pub(crate) fn adapter_value_variant(value: &AdapterValue) -> Variant {
    match value {
        AdapterValue::Identifier(value) | AdapterValue::String(value) => value.to_variant(),
        AdapterValue::Integer(value) => value.to_variant(),
        AdapterValue::Float(value) => value.to_variant(),
        AdapterValue::Boolean(value) => value.to_variant(),
    }
}

pub(crate) fn adapter_value_dictionary(value: &AdapterValue) -> VarDictionary {
    let mut dictionary = VarDictionary::new();
    dictionary.set("kind", adapter_value_kind(value));
    set_variant(&mut dictionary, "value", adapter_value_variant(value));
    dictionary
}

fn adapter_value_kind(value: &AdapterValue) -> &'static str {
    match value {
        AdapterValue::Identifier(_) => "identifier",
        AdapterValue::String(_) => "string",
        AdapterValue::Integer(_) => "integer",
        AdapterValue::Float(_) => "float",
        AdapterValue::Boolean(_) => "boolean",
    }
}

fn line_dictionary(line: &DialogueLine) -> VarDictionary {
    let mut dictionary = VarDictionary::new();
    dictionary.set("id", line.id.as_str());
    dictionary.set("source_text", line.source_text.as_str());
    dictionary.set("text", line.text.as_str());
    set_variant(
        &mut dictionary,
        "speaker",
        line.speaker
            .as_ref()
            .map_or_else(Variant::nil, |speaker| speaker.as_str().to_variant()),
    );
    set_variant(
        &mut dictionary,
        "metadata",
        metadata_array(&line.metadata).to_variant(),
    );
    dictionary
}

fn choices_array(choices: &[DialogueChoice]) -> VarArray {
    let mut array = VarArray::new();
    for choice in choices {
        push_variant(&mut array, choice_dictionary(choice).to_variant());
    }
    array
}

fn choice_dictionary(choice: &DialogueChoice) -> VarDictionary {
    let mut dictionary = VarDictionary::new();
    dictionary.set("id", choice.id.as_str());
    dictionary.set("source_text", choice.source_text.as_str());
    dictionary.set("text", choice.text.as_str());
    set_variant(
        &mut dictionary,
        "metadata",
        metadata_array(&choice.metadata).to_variant(),
    );
    set_variant(
        &mut dictionary,
        "availability",
        availability_dictionary(&choice.availability).to_variant(),
    );
    let (echo, line_id) = choice_echo_name(&choice.echo);
    dictionary.set("echo", echo);
    set_variant(
        &mut dictionary,
        "echo_line_id",
        line_id.map_or_else(Variant::nil, |line_id| line_id.to_variant()),
    );
    dictionary
}

fn availability_dictionary(availability: &ChoiceAvailability) -> VarDictionary {
    let (is_available, has_primary_reason, has_reason_tree) = availability_summary(availability);
    let mut dictionary = VarDictionary::new();
    dictionary.set("is_available", is_available);
    dictionary.set("has_primary_reason", has_primary_reason);
    dictionary.set("has_reason_tree", has_reason_tree);
    set_variant(
        &mut dictionary,
        "primary_reason",
        availability
            .primary_reason
            .as_ref()
            .map_or_else(Variant::nil, |reason| {
                reason_dictionary(reason).to_variant()
            }),
    );
    set_variant(
        &mut dictionary,
        "reason_tree",
        availability
            .reason_tree
            .as_ref()
            .map_or_else(Variant::nil, |tree| {
                reason_tree_dictionary(tree).to_variant()
            }),
    );
    dictionary
}

fn reason_tree_dictionary(tree: &ChoiceAvailabilityReasonTree) -> VarDictionary {
    let mut dictionary = VarDictionary::new();
    dictionary.set("kind", reason_tree_kind(tree));
    match tree {
        ChoiceAvailabilityReasonTree::All(children)
        | ChoiceAvailabilityReasonTree::Any(children) => {
            let mut array = VarArray::new();
            for child in children {
                push_variant(&mut array, reason_tree_dictionary(child).to_variant());
            }
            set_variant(&mut dictionary, "children", array.to_variant());
        }
        ChoiceAvailabilityReasonTree::Reason(reason) => {
            set_variant(
                &mut dictionary,
                "reason",
                reason_dictionary(reason).to_variant(),
            );
        }
        ChoiceAvailabilityReasonTree::RequirementSourceText(source_text) => {
            dictionary.set("source_text", source_text.as_str());
        }
    }
    dictionary
}

fn reason_dictionary(reason: &ChoiceAvailabilityReason) -> VarDictionary {
    let (id, source_text, text) = reason_id_text(reason);
    let mut dictionary = VarDictionary::new();
    dictionary.set("id", id);
    dictionary.set("source_text", source_text);
    dictionary.set("text", text);
    set_variant(
        &mut dictionary,
        "args",
        reason_args_array(reason).to_variant(),
    );
    set_variant(
        &mut dictionary,
        "origin",
        reason.origin.as_ref().map_or_else(Variant::nil, |origin| {
            origin_dictionary(origin).to_variant()
        }),
    );
    dictionary
}

fn reason_args_array(reason: &ChoiceAvailabilityReason) -> VarArray {
    let mut array = VarArray::new();
    for arg in &reason.args {
        let (name, value) = reason_arg_parts(arg);
        let mut dictionary = adapter_value_dictionary(&value);
        dictionary.set("name", name);
        push_variant(&mut array, dictionary.to_variant());
    }
    array
}

fn origin_dictionary(origin: &ChoiceAvailabilityReasonOrigin) -> VarDictionary {
    let mut dictionary = VarDictionary::new();
    dictionary.set("kind", reason_origin_kind(origin));
    match origin {
        ChoiceAvailabilityReasonOrigin::ConditionCall { function, args } => {
            dictionary.set("function", function.as_str());
            let mut array = VarArray::new();
            for arg in args {
                push_variant(
                    &mut array,
                    adapter_value_dictionary(&reason_value_parts(arg)).to_variant(),
                );
            }
            set_variant(&mut dictionary, "args", array.to_variant());
        }
        ChoiceAvailabilityReasonOrigin::RequirementExpression { source_text } => {
            dictionary.set("source_text", source_text.as_str());
        }
    }
    dictionary
}

fn effects_array(effects: &[DialogueEffectRequest]) -> VarArray {
    let mut array = VarArray::new();
    for effect in effects {
        push_variant(&mut array, effect_dictionary(effect).to_variant());
    }
    array
}

fn effect_dictionary(effect: &DialogueEffectRequest) -> VarDictionary {
    let mut dictionary = VarDictionary::new();
    dictionary.set("id", effect.id.as_str());
    dictionary.set("mode", effect_mode_name(effect.mode));
    dictionary.set("function", effect.function.as_str());
    set_variant(
        &mut dictionary,
        "args",
        effect_args_array(effect).to_variant(),
    );
    set_variant(
        &mut dictionary,
        "source_span",
        source_span_dictionary(&effect.source_span).to_variant(),
    );
    dictionary
}

fn effect_args_array(effect: &DialogueEffectRequest) -> VarArray {
    let mut array = VarArray::new();
    for argument in &effect.args {
        let value = effect_arg_value(argument);
        push_variant(&mut array, adapter_value_dictionary(&value).to_variant());
    }
    array
}

fn metadata_array(entries: &[recite_core::MetadataEntry]) -> VarArray {
    let mut array = VarArray::new();
    for (key, value) in metadata_entries(entries) {
        let mut dictionary = VarDictionary::new();
        dictionary.set("key", key);
        set_variant(
            &mut dictionary,
            "value",
            value_dictionary(value).to_variant(),
        );
        push_variant(&mut array, dictionary.to_variant());
    }
    array
}

fn value_dictionary(value: &Value) -> VarDictionary {
    let mut dictionary = VarDictionary::new();
    match value {
        Value::Scalar(value) => {
            dictionary.set("kind", scalar_value_kind(value));
            set_variant(&mut dictionary, "value", scalar_variant(value));
        }
        Value::Array(values) => {
            dictionary.set("kind", "array");
            let mut array = VarArray::new();
            for value in values {
                push_variant(&mut array, scalar_variant(value));
            }
            set_variant(&mut dictionary, "value", array.to_variant());
        }
    }
    dictionary
}

fn scalar_variant(value: &ScalarValue) -> Variant {
    match value {
        ScalarValue::String(value) => value.to_variant(),
        ScalarValue::Integer(value) => value.to_variant(),
        ScalarValue::Float(value) => value.to_variant(),
        ScalarValue::Boolean(value) => value.to_variant(),
    }
}

fn source_span_dictionary(span: &recite_core::SourceSpan) -> VarDictionary {
    let (file, start_line, start_column, end) = source_span_parts(span);
    let mut dictionary = VarDictionary::new();
    dictionary.set("file", file);
    dictionary.set("start_line", i64::from(start_line));
    dictionary.set("start_column", i64::from(start_column));
    if let Some((end_line, end_column)) = end {
        dictionary.set("end_line", i64::from(end_line));
        dictionary.set("end_column", i64::from(end_column));
    } else {
        set_variant(&mut dictionary, "end_line", Variant::nil());
        set_variant(&mut dictionary, "end_column", Variant::nil());
    }
    dictionary
}

fn set_variant(dictionary: &mut VarDictionary, key: &str, value: Variant) {
    dictionary.set(key, &value);
}

fn push_variant(array: &mut VarArray, value: Variant) {
    array.push(&value);
}
