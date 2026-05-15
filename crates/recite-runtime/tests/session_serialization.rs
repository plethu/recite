use recite_compiler::{CompileInput, CompileOptions, compile_inputs};
use recite_core::{
    ChoiceId, CompiledAssetId, CompiledDialogue, CompilerVersion, LocaleId, SchemaFingerprint,
    SourceMapId,
};
use recite_runtime::{
    DialogueDeferredEffectSnapshot, DialogueEffectArgument, DialogueEffectRequest, DialogueError,
    DialogueEvent, DialogueSessionOptions, EmptyDialogueContext, choose as runtime_choose,
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
#[path = "session_serialization/pending_prompt.rs"]
mod pending_prompt;
#[path = "session_serialization/round_trip.rs"]
mod round_trip;
#[path = "session_serialization/support.rs"]
mod support;

use support::*;
