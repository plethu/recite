use recite_runtime::{
    ChoiceAvailabilityReason, ChoiceAvailabilityReasonOrigin, ChoiceAvailabilityReasonTree,
    ChoiceAvailabilityReasonValue, ChoiceEchoMode, DialogueChoice, DialogueLine, PreviewPrompt,
    PreviewPromptIdentity,
};

use crate::preview_hash_plural::hash_plural;
use crate::preview_hash_primitives::{
    hash_bool, hash_i64, hash_len, hash_optional_span, hash_optional_text, hash_optional_u64,
    hash_text, hash_u64, hash_value, tag,
};

pub(super) fn hash_optional_prompt(
    hasher: &mut blake3::Hasher,
    prompt: Option<&PreviewPromptIdentity>,
) {
    if let Some(prompt) = prompt {
        tag(hasher, 1);
        hash_identity(hasher, prompt);
    } else {
        tag(hasher, 0);
    }
}

pub(super) fn hash_identity(hasher: &mut blake3::Hasher, identity: &PreviewPromptIdentity) {
    hash_text(hasher, identity.block().as_str());
    hash_optional_text(hasher, identity.line().map(recite_core::LineId::as_str));
    hash_len(hasher, identity.choices().len());
    for choice in identity.choices() {
        hash_text(hasher, choice.as_str());
    }
}

pub(super) fn hash_prompt(hasher: &mut blake3::Hasher, prompt: &PreviewPrompt) {
    hash_identity(hasher, prompt.identity());
    hash_optional_u64(
        hasher,
        recite_runtime::bench_support::plural_arm_count(prompt).map(|count| count as u64),
    );
    if let Some(line) = prompt.line() {
        tag(hasher, 1);
        hash_line(hasher, line);
    } else {
        tag(hasher, 0);
    }
    hash_len(hasher, prompt.choices().len());
    for choice in prompt.choices() {
        hash_choice(hasher, choice);
    }
}

pub(super) fn hash_line(hasher: &mut blake3::Hasher, line: &DialogueLine) {
    hash_text(hasher, line.id.as_str());
    hash_text(hasher, &line.source_text);
    hash_text(hasher, &line.text);
    hash_optional_text(
        hasher,
        line.speaker.as_ref().map(recite_core::SpeakerId::as_str),
    );
    hash_len(hasher, line.metadata.len());
    for metadata in &line.metadata {
        hash_text(hasher, &metadata.key);
        hash_value(hasher, &metadata.value);
        hash_optional_span(hasher, metadata.source_span.as_ref());
        hash_optional_span(hasher, metadata.key_span.as_ref());
        hash_optional_span(hasher, metadata.value_span.as_ref());
    }
    if let Some(plural) = &line.plural {
        tag(hasher, 1);
        hash_plural(hasher, plural);
    } else {
        tag(hasher, 0);
    }
}

pub(crate) fn hash_choice(hasher: &mut blake3::Hasher, choice: &DialogueChoice) {
    hash_text(hasher, choice.id.as_str());
    hash_text(hasher, &choice.source_text);
    hash_text(hasher, &choice.text);
    hash_bool(hasher, choice.availability.is_available);
    hash_optional_reason(hasher, choice.availability.primary_reason.as_ref());
    hash_optional_reason_tree(hasher, choice.availability.reason_tree.as_ref());
    hash_len(hasher, choice.metadata.len());
    for metadata in &choice.metadata {
        hash_text(hasher, &metadata.key);
        hash_value(hasher, &metadata.value);
        hash_optional_span(hasher, metadata.source_span.as_ref());
        hash_optional_span(hasher, metadata.key_span.as_ref());
        hash_optional_span(hasher, metadata.value_span.as_ref());
    }
    match &choice.echo {
        ChoiceEchoMode::None => tag(hasher, 0),
        ChoiceEchoMode::SelectedText => tag(hasher, 1),
        ChoiceEchoMode::ExplicitLine(line) => {
            tag(hasher, 2);
            hash_text(hasher, line.as_str());
        }
    }
}

pub(super) fn hash_reason(hasher: &mut blake3::Hasher, reason: &ChoiceAvailabilityReason) {
    hash_text(hasher, reason.id.as_str());
    hash_text(hasher, &reason.source_text);
    hash_text(hasher, &reason.text);
    match &reason.origin {
        Some(ChoiceAvailabilityReasonOrigin::ConditionCall { function, args }) => {
            tag(hasher, 1);
            hash_text(hasher, function);
            hash_len(hasher, args.len());
            for arg in args {
                hash_reason_value(hasher, arg);
            }
        }
        Some(ChoiceAvailabilityReasonOrigin::RequirementExpression { source_text }) => {
            tag(hasher, 2);
            hash_text(hasher, source_text);
        }
        None => tag(hasher, 0),
    }
    hash_len(hasher, reason.args.len());
    for arg in &reason.args {
        hash_text(hasher, &arg.name);
        hash_reason_value(hasher, &arg.value);
    }
}

fn hash_reason_value(hasher: &mut blake3::Hasher, value: &ChoiceAvailabilityReasonValue) {
    match value {
        ChoiceAvailabilityReasonValue::Identifier(value) => {
            tag(hasher, 0);
            hash_text(hasher, value);
        }
        ChoiceAvailabilityReasonValue::String(value) => {
            tag(hasher, 1);
            hash_text(hasher, value);
        }
        ChoiceAvailabilityReasonValue::Integer(value) => {
            tag(hasher, 2);
            hash_i64(hasher, *value);
        }
        ChoiceAvailabilityReasonValue::Float(value) => {
            tag(hasher, 3);
            hash_u64(hasher, value.to_bits());
        }
        ChoiceAvailabilityReasonValue::Boolean(value) => {
            tag(hasher, 4);
            hash_bool(hasher, *value);
        }
    }
}

pub(super) fn hash_optional_reason(
    hasher: &mut blake3::Hasher,
    reason: Option<&ChoiceAvailabilityReason>,
) {
    if let Some(reason) = reason {
        tag(hasher, 1);
        hash_reason(hasher, reason);
    } else {
        tag(hasher, 0);
    }
}

pub(super) fn hash_optional_reason_tree(
    hasher: &mut blake3::Hasher,
    tree: Option<&ChoiceAvailabilityReasonTree>,
) {
    if let Some(tree) = tree {
        tag(hasher, 1);
        hash_reason_tree(hasher, tree);
    } else {
        tag(hasher, 0);
    }
}

fn hash_reason_tree(hasher: &mut blake3::Hasher, tree: &ChoiceAvailabilityReasonTree) {
    match tree {
        ChoiceAvailabilityReasonTree::All(children) => {
            tag(hasher, 0);
            hash_reason_tree_children(hasher, children);
        }
        ChoiceAvailabilityReasonTree::Any(children) => {
            tag(hasher, 1);
            hash_reason_tree_children(hasher, children);
        }
        ChoiceAvailabilityReasonTree::Reason(reason) => {
            tag(hasher, 2);
            hash_reason(hasher, reason);
        }
        ChoiceAvailabilityReasonTree::RequirementSourceText(text) => {
            tag(hasher, 3);
            hash_text(hasher, text);
        }
    }
}

fn hash_reason_tree_children(
    hasher: &mut blake3::Hasher,
    children: &[ChoiceAvailabilityReasonTree],
) {
    hash_len(hasher, children.len());
    for child in children {
        hash_reason_tree(hasher, child);
    }
}

pub(super) fn hash_availability(
    hasher: &mut blake3::Hasher,
    availability: &recite_runtime::ChoiceAvailability,
) {
    hash_bool(hasher, availability.is_available);
    hash_optional_reason(hasher, availability.primary_reason.as_ref());
    hash_optional_reason_tree(hasher, availability.reason_tree.as_ref());
}
