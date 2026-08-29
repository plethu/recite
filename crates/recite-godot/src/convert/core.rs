use godot::builtin::{PackedByteArray, VarArray, VarDictionary, Variant};
use godot::prelude::ToGodot;

use crate::adapter::{AdapterError, AdapterValue, ReciteOutput};

use super::effects::effects_array;
use super::line::{choices_array, line_dictionary};

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
                super::effects::effect_dictionary(effect).to_variant(),
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

pub(crate) fn adapter_value_dictionary(value: &AdapterValue) -> VarDictionary {
    super::values::adapter_value_dictionary(value)
}

pub(super) fn set_variant(dictionary: &mut VarDictionary, key: &str, value: Variant) {
    dictionary.set(key, &value);
}

pub(super) fn push_variant(array: &mut VarArray, value: Variant) {
    array.push(&value);
}
