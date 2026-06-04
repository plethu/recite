#![cfg(test)]

use recite_core::{ChoiceId, CompiledDialogue, EffectId, LocaleId};
use recite_runtime::{
    DialogueChoiceAvailabilityReasonTreeSnapshot, DialogueDeferredEffectSnapshot,
    DialogueEffectArgument, DialogueEffectRequest, DialogueError, DialogueEvent,
    DialogueSessionOptions, DialogueSessionPendingEffectSnapshot, EffectAck, acknowledge_effect,
    decode_session_messagepack, encode_session_messagepack, next as runtime_next, restore_session,
    snapshot_session, start_scene, start_scene_with_options,
};

#[path = "session_serialization/asset_identity.rs"]
mod asset_identity;
#[path = "session_serialization/continuation.rs"]
mod continuation;
#[path = "session_serialization/deferred_effects.rs"]
mod deferred_effects;
#[path = "session_serialization/ended_state.rs"]
mod ended_state;
#[path = "session_serialization/invalid_snapshots.rs"]
mod invalid_snapshots;
#[path = "session_serialization/pending_effect.rs"]
mod pending_effect;
#[path = "session_serialization/pending_prompt.rs"]
mod pending_prompt;
#[path = "session_serialization/round_trip.rs"]
mod round_trip;
#[path = "support/shared.rs"]
mod shared_support;
#[path = "session_serialization/support.rs"]
mod support;

use shared_support::*;
use support::*;
