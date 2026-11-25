#![cfg(test)]

use recite_core::{ChoiceId, EffectId, LocaleId, compiled::CompiledDialogue};
use recite_runtime::{
    DialogueEffectArgument, DialogueEffectRequest, DialogueError, DialogueEvent,
    DialogueSessionOptions, EffectAck, acknowledge_effect, next as runtime_next,
    snapshot::{
        DialogueChoiceAvailabilityReasonOriginSnapshot, DialogueChoiceAvailabilityReasonSnapshot,
        DialogueChoiceAvailabilityReasonTreeSnapshot,
        DialogueChoiceAvailabilityReasonValueSnapshot, DialogueEffectModeSnapshot,
        DialogueEffectRequestSnapshot, DialogueSessionPendingEffectSnapshot,
        DialogueSessionSnapshot, DialogueSessionSnapshotConversionError,
        DialogueSourceSpanSnapshot, decode_session_messagepack, encode_session_messagepack,
        restore_session, snapshot_session,
    },
    start_scene, start_scene_with_options,
};

#[path = "session_serialization/asset_identity.rs"]
mod asset_identity;
#[path = "session_serialization/availability_conversion.rs"]
mod availability_conversion;
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

/// A plausible saved effect request that no compiled effect produced.
fn forged_effect_request(
    id: &str,
    mode: DialogueEffectModeSnapshot,
) -> DialogueEffectRequestSnapshot {
    DialogueEffectRequestSnapshot {
        id: id.to_owned(),
        mode,
        function: "forged".to_owned(),
        args: Vec::new(),
        source_span: DialogueSourceSpanSnapshot {
            file: "dialogue/start.recite".to_owned(),
            start_line: 2,
            start_column: 1,
            end: None,
        },
    }
}
