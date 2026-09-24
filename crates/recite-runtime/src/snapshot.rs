//! Public snapshot capability over runtime internals.

pub use crate::session_serialization::{
    decode_session_messagepack, encode_session_messagepack, restore_session,
};
pub use crate::session_snapshot::{
    CURRENT_SESSION_SNAPSHOT_FORMAT_VERSION, DialogueChoiceAvailabilityReasonArgSnapshot,
    DialogueChoiceAvailabilityReasonOriginSnapshot, DialogueChoiceAvailabilityReasonSnapshot,
    DialogueChoiceAvailabilityReasonTreeSnapshot, DialogueChoiceAvailabilityReasonValueSnapshot,
    DialogueChoiceAvailabilitySnapshot, DialogueContentFingerprintSnapshot,
    DialogueDeferredEffectSnapshot, DialogueSchemaFingerprintSnapshot,
    DialogueSessionFrameSnapshot, DialogueSessionPendingChoiceSnapshot,
    DialogueSessionPendingEffectSnapshot, DialogueSessionPendingPromptSnapshot,
    DialogueSessionRangeSnapshot, DialogueSessionSnapshot, DialogueSessionSnapshotConversionError,
    DialogueSessionSourceSnapshot, snapshot_session,
};
