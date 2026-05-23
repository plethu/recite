//! Deterministic Recite runtime with no engine dependencies.

mod context;
mod error;
mod event;
mod session;
mod session_serialization;
mod session_snapshot;
mod traversal;

pub use context::{
    ConditionArgument, ConditionArguments, ConditionEvaluationError, ConditionQuery,
    DialogueContext, EmptyDialogueContext,
};
pub use error::{DialogueError, UnsupportedStatementKind};
pub use event::{
    ChoiceEchoMode, DialogueChoice, DialogueEffectArgument, DialogueEffectMode,
    DialogueEffectRequest, DialogueEvent, DialogueLine, EffectAck,
};
pub use session::{DialogueSession, DialogueSessionOptions};
pub use session_serialization::{
    decode_session_messagepack, encode_session_messagepack, restore_session,
};
pub use session_snapshot::{
    DialogueContentFingerprintSnapshot, DialogueDeferredEffectSnapshot,
    DialogueSchemaFingerprintSnapshot, DialogueSessionFrameSnapshot,
    DialogueSessionPendingChoiceSnapshot, DialogueSessionPendingPromptSnapshot,
    DialogueSessionRangeSnapshot, DialogueSessionSnapshot, DialogueSessionSourceSnapshot,
    SESSION_SNAPSHOT_FORMAT_VERSION_V0, snapshot_session,
};
pub use traversal::{acknowledge_effect, choose, next, start_scene, start_scene_with_options};
