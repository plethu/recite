use recite_runtime::{
    ConditionValue, DialogueEffectArgument, DialogueEffectRequest, DialogueLine,
    PreviewConditionArgument, PreviewConditionResult, PreviewEvent, PreviewPrompt,
    PreviewPromptIdentity,
};

use crate::preview_hash_primitives::{
    hash_bool, hash_i64, hash_len, hash_optional_text, hash_text, hash_u64, tag,
};

/// Hashes stable structured event payloads without debug/display formatting.
/// This is a traversal regression key, not a serialized compatibility format.
pub(crate) fn hash_event(event: &PreviewEvent, hasher: &mut blake3::Hasher) {
    match event {
        PreviewEvent::ConditionRequested(request) => {
            tag(hasher, 1);
            hash_u64(hasher, request.id().get());
            hash_text(hasher, request.block().as_str());
            hash_optional_prompt(hasher, request.prompt());
            hash_query(hasher, request.query());
        }
        PreviewEvent::ConditionResult { request, result } => {
            tag(hasher, 2);
            hash_u64(hasher, request.id().get());
            hash_query(hasher, request.query());
            hash_result(hasher, result);
        }
        PreviewEvent::Line(line) => {
            tag(hasher, 3);
            hash_line(hasher, line);
        }
        PreviewEvent::Prompt(prompt) => {
            tag(hasher, 4);
            hash_prompt(hasher, prompt);
        }
        PreviewEvent::ChoiceAccepted { prompt, choice_id } => {
            tag(hasher, 5);
            hash_identity(hasher, prompt);
            hash_text(hasher, choice_id.as_str());
        }
        PreviewEvent::ChoiceSelected { prompt, choice_id } => {
            tag(hasher, 6);
            hash_identity(hasher, prompt);
            hash_text(hasher, choice_id.as_str());
        }
        PreviewEvent::EffectRequested(effect) => {
            tag(hasher, 7);
            hash_effect(hasher, effect);
        }
        PreviewEvent::DeferredEffectScheduled(effect) => {
            tag(hasher, 8);
            hash_effect(hasher, effect);
        }
        PreviewEvent::EffectAcknowledged { effect_id, ack } => {
            tag(hasher, 9);
            hash_text(hasher, effect_id.as_str());
            match ack {
                recite_runtime::EffectAck::Completed => tag(hasher, 0),
                recite_runtime::EffectAck::Failed { reason } => {
                    tag(hasher, 1);
                    hash_text(hasher, reason);
                }
            }
        }
        PreviewEvent::End { deferred_effects } => {
            tag(hasher, 10);
            hash_len(hasher, deferred_effects.len());
            for effect in deferred_effects {
                hash_effect(hasher, effect);
            }
        }
        PreviewEvent::Restarted { block, locale } => {
            tag(hasher, 11);
            hash_optional_text(hasher, block.as_ref().map(recite_core::BlockId::as_str));
            hash_optional_text(hasher, locale.as_ref().map(recite_core::LocaleId::as_str));
        }
        PreviewEvent::Restored => tag(hasher, 12),
        PreviewEvent::RestartRequired { .. } => tag(hasher, 13),
        PreviewEvent::Error(_) => tag(hasher, 14),
        _ => tag(hasher, 255),
    }
}

fn hash_query(hasher: &mut blake3::Hasher, query: &recite_runtime::PreviewConditionQuery) {
    hash_text(hasher, query.function());
    tag(
        hasher,
        match query.expected_type() {
            recite_runtime::ConditionExpectedType::Bool => 0,
            recite_runtime::ConditionExpectedType::Enum => 1,
        },
    );
    hash_len(hasher, query.arguments().len());
    for argument in query.arguments() {
        match argument {
            PreviewConditionArgument::Identifier(value) => {
                tag(hasher, 0);
                hash_text(hasher, value);
            }
            PreviewConditionArgument::String(value) => {
                tag(hasher, 1);
                hash_text(hasher, value);
            }
            PreviewConditionArgument::Integer(value) => {
                tag(hasher, 2);
                hash_i64(hasher, *value);
            }
            PreviewConditionArgument::Float(value) => {
                tag(hasher, 3);
                hash_u64(hasher, value.to_bits());
            }
            PreviewConditionArgument::Boolean(value) => {
                tag(hasher, 4);
                hash_bool(hasher, *value);
            }
            _ => tag(hasher, 255),
        }
    }
}

fn hash_result(hasher: &mut blake3::Hasher, result: &PreviewConditionResult) {
    match result {
        PreviewConditionResult::Value(ConditionValue::Bool(value)) => {
            tag(hasher, 0);
            hash_bool(hasher, *value);
        }
        PreviewConditionResult::Value(ConditionValue::EnumVariant(value)) => {
            tag(hasher, 1);
            hash_text(hasher, value);
        }
        PreviewConditionResult::Failed { reason } => {
            tag(hasher, 2);
            hash_text(hasher, reason);
        }
        _ => tag(hasher, 255),
    }
}

fn hash_optional_prompt(hasher: &mut blake3::Hasher, prompt: Option<&PreviewPromptIdentity>) {
    if let Some(prompt) = prompt {
        tag(hasher, 1);
        hash_identity(hasher, prompt);
    } else {
        tag(hasher, 0);
    }
}

fn hash_identity(hasher: &mut blake3::Hasher, identity: &PreviewPromptIdentity) {
    hash_text(hasher, identity.block().as_str());
    hash_optional_text(hasher, identity.line().map(recite_core::LineId::as_str));
    hash_len(hasher, identity.choices().len());
    for choice in identity.choices() {
        hash_text(hasher, choice.as_str());
    }
}

fn hash_prompt(hasher: &mut blake3::Hasher, prompt: &PreviewPrompt) {
    hash_identity(hasher, prompt.identity());
    if let Some(line) = prompt.line() {
        tag(hasher, 1);
        hash_line(hasher, line);
    } else {
        tag(hasher, 0);
    }
    hash_len(hasher, prompt.choices().len());
    for choice in prompt.choices() {
        hash_text(hasher, choice.id.as_str());
        hash_text(hasher, &choice.source_text);
        hash_text(hasher, &choice.text);
        hash_bool(hasher, choice.availability.is_available);
        hash_len(hasher, choice.metadata.len());
    }
}

fn hash_line(hasher: &mut blake3::Hasher, line: &DialogueLine) {
    hash_text(hasher, line.id.as_str());
    hash_text(hasher, &line.source_text);
    hash_text(hasher, &line.text);
    hash_optional_text(
        hasher,
        line.speaker.as_ref().map(recite_core::SpeakerId::as_str),
    );
    hash_len(hasher, line.metadata.len());
    if let Some(plural) = &line.plural {
        tag(hasher, 1);
        hash_text(hasher, &plural.singular_source_text);
        hash_text(hasher, &plural.plural_source_text);
        hash_i64(hasher, plural.count);
        hash_u64(hasher, plural.selected_arm as u64);
    } else {
        tag(hasher, 0);
    }
}

fn hash_effect(hasher: &mut blake3::Hasher, effect: &DialogueEffectRequest) {
    hash_text(hasher, effect.id.as_str());
    tag(
        hasher,
        match effect.mode {
            recite_runtime::DialogueEffectMode::Deferred => 0,
            recite_runtime::DialogueEffectMode::Immediate => 1,
            recite_runtime::DialogueEffectMode::Blocking => 2,
        },
    );
    hash_text(hasher, &effect.function);
    hash_len(hasher, effect.args.len());
    for argument in &effect.args {
        match argument {
            DialogueEffectArgument::Identifier(value) => {
                tag(hasher, 0);
                hash_text(hasher, value);
            }
            DialogueEffectArgument::String(value) => {
                tag(hasher, 1);
                hash_text(hasher, value);
            }
            DialogueEffectArgument::Integer(value) => {
                tag(hasher, 2);
                hash_i64(hasher, *value);
            }
            DialogueEffectArgument::Float(value) => {
                tag(hasher, 3);
                hash_u64(hasher, value.to_bits());
            }
            DialogueEffectArgument::Boolean(value) => {
                tag(hasher, 4);
                hash_bool(hasher, *value);
            }
        }
    }
}
