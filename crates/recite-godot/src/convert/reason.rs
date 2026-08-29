use godot::builtin::{VarArray, VarDictionary, Variant};
use godot::prelude::ToGodot;

use recite_runtime::{
    ChoiceAvailability, ChoiceAvailabilityReason, ChoiceAvailabilityReasonOrigin,
    ChoiceAvailabilityReasonTree,
};

use super::core::{push_variant, set_variant};
use super::values::adapter_value_dictionary;
use crate::adapter_model::{
    availability_summary, reason_arg_parts, reason_id_text, reason_origin_kind, reason_tree_kind,
    reason_value_parts,
};

pub(super) fn availability_dictionary(availability: &ChoiceAvailability) -> VarDictionary {
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
