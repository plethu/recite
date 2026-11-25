//! Public snapshot capability over runtime internals.
//!
//! [`restore_session`] and [`decode_session_messagepack`] restore against the
//! exact asset a session was saved against. To load saves after a patch,
//! use [`crate::resume`].

pub use crate::session_serialization::{
    decode_session_messagepack, decode_session_snapshot_messagepack, encode_session_messagepack,
    restore_session,
};
pub use crate::session_snapshot::{
    CURRENT_SESSION_SNAPSHOT_FORMAT_VERSION, DialogueBlockDigestsSnapshot, DialogueBranchSnapshot,
    DialogueChoiceAvailabilityReasonArgSnapshot, DialogueChoiceAvailabilityReasonOriginSnapshot,
    DialogueChoiceAvailabilityReasonSnapshot, DialogueChoiceAvailabilityReasonTreeSnapshot,
    DialogueChoiceAvailabilityReasonValueSnapshot, DialogueChoiceAvailabilitySnapshot,
    DialogueContentFingerprintSnapshot, DialogueEffectArgumentSnapshot, DialogueEffectModeSnapshot,
    DialogueEffectRequestSnapshot, DialogueLandmarkSnapshot, DialoguePositionAnchorSnapshot,
    DialogueResumeAnchorSnapshot, DialogueResumeFrameSnapshot, DialogueSchemaFingerprintSnapshot,
    DialogueSessionFrameSnapshot, DialogueSessionPendingChoiceSnapshot,
    DialogueSessionPendingEffectSnapshot, DialogueSessionPendingPromptSnapshot,
    DialogueSessionRangeSnapshot, DialogueSessionSnapshot, DialogueSessionSnapshotConversionError,
    DialogueSessionSourceSnapshot, DialogueSourcePositionSnapshot, DialogueSourceSpanSnapshot,
    DialogueStatementSignatureSnapshot, effect_request_from_snapshot, effect_request_snapshot,
    snapshot_session,
};
