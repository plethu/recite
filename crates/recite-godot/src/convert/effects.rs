use godot::builtin::{VarArray, VarDictionary};
use godot::prelude::ToGodot;

use recite_core::{MetadataEntry, SourceSpan};
use recite_runtime::DialogueEffectRequest;

use crate::adapter_model::{
    effect_arg_value, effect_mode_name, metadata_entries, source_span_parts,
};

use super::core::{push_variant, set_variant};
use super::values::value_dictionary;

pub(super) fn effects_array(effects: &[DialogueEffectRequest]) -> VarArray {
    let mut array = VarArray::new();
    for effect in effects {
        push_variant(&mut array, effect_dictionary(effect).to_variant());
    }
    array
}

pub(super) fn effect_dictionary(effect: &DialogueEffectRequest) -> VarDictionary {
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
        push_variant(
            &mut array,
            super::values::adapter_value_dictionary(&value).to_variant(),
        );
    }
    array
}

fn source_span_dictionary(span: &SourceSpan) -> VarDictionary {
    let (file, start_line, start_column, end) = source_span_parts(span);
    let mut dictionary = VarDictionary::new();
    dictionary.set("file", file);
    dictionary.set("start_line", i64::from(start_line));
    dictionary.set("start_column", i64::from(start_column));
    if let Some((end_line, end_column)) = end {
        dictionary.set("end_line", i64::from(end_line));
        dictionary.set("end_column", i64::from(end_column));
    } else {
        set_variant(&mut dictionary, "end_line", godot::builtin::Variant::nil());
        set_variant(
            &mut dictionary,
            "end_column",
            godot::builtin::Variant::nil(),
        );
    }
    dictionary
}

pub(super) fn metadata_array(entries: &[MetadataEntry]) -> VarArray {
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
