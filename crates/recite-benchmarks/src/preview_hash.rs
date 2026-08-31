use recite_runtime::{PreviewEvent, PreviewOutput, PreviewState, PreviewStatus};

use crate::preview_hash_dialogue::{hash_identity, hash_line, hash_prompt};
use crate::preview_hash_errors::hash_preview_error;
use crate::preview_hash_primitives::{hash_len, hash_optional_text, hash_text, tag};
use crate::preview_hash_runtime::{hash_effect, hash_request, hash_result, hash_revision};

/// Hashes one command boundary's complete structured preview state. The
/// boundary marker keeps adjacent outputs distinct from event evidence.
pub(crate) fn hash_output_state(output: &PreviewOutput, hasher: &mut blake3::Hasher) {
    tag(hasher, 0x20);
    hash_state(output.state(), hasher);
}

/// Hashes every stable, structured field exposed by a preview state. This is a
/// regression key, not a serialized compatibility format.
pub(crate) fn hash_state(state: &PreviewState, hasher: &mut blake3::Hasher) {
    hash_text(hasher, state.asset_id().as_str());
    hash_optional_text(hasher, state.block().map(recite_core::BlockId::as_str));
    hash_optional_text(hasher, state.locale().map(recite_core::LocaleId::as_str));
    hash_status(hasher, state.status());
    hash_len(hasher, state.selected_choice_history().len());
    for choice_id in state.selected_choice_history() {
        hash_text(hasher, choice_id.as_str());
    }
    hash_len(hasher, state.deferred_effects().len());
    for effect in state.deferred_effects() {
        hash_effect(hasher, effect);
    }
    if let Some(requirement) = state.restart_required() {
        tag(hasher, 1);
        hash_text(hasher, requirement.active_asset().as_str());
        hash_text(hasher, requirement.replacement_asset().as_str());
        hash_revision(hasher, requirement.active_revision());
        hash_revision(hasher, requirement.replacement_revision());
    } else {
        tag(hasher, 0);
    }
}

fn hash_status(hasher: &mut blake3::Hasher, status: &PreviewStatus) {
    match status {
        PreviewStatus::Ready => tag(hasher, 0),
        PreviewStatus::WaitingForCondition { request } => {
            tag(hasher, 1);
            hash_request(hasher, request);
        }
        PreviewStatus::WaitingForChoice { prompt } => {
            tag(hasher, 2);
            hash_prompt(hasher, prompt);
        }
        PreviewStatus::WaitingForEffect { effect } => {
            tag(hasher, 3);
            hash_effect(hasher, effect);
        }
        PreviewStatus::Ended => tag(hasher, 4),
        _ => tag(hasher, 255),
    }
}

/// Hashes every stable, structured field exposed by a preview event. The
/// digest is a regression key, not a serialized compatibility format.
pub(crate) fn hash_event(event: &PreviewEvent, hasher: &mut blake3::Hasher) {
    match event {
        PreviewEvent::ConditionRequested(request) => {
            tag(hasher, 1);
            hash_request(hasher, request);
        }
        PreviewEvent::ConditionResult { request, result } => {
            tag(hasher, 2);
            hash_request(hasher, request);
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
        PreviewEvent::RestartRequired {
            active_asset,
            replacement_asset,
            active_revision,
            replacement_revision,
        } => {
            tag(hasher, 13);
            hash_text(hasher, active_asset.as_str());
            hash_text(hasher, replacement_asset.as_str());
            hash_revision(hasher, active_revision);
            hash_revision(hasher, replacement_revision);
        }
        PreviewEvent::Error(error) => {
            tag(hasher, 14);
            hash_preview_error(hasher, error);
        }
        _ => tag(hasher, 255),
    }
}
