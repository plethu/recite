use recite_core::CompiledAssetId;

use crate::{ConditionExpectedType, DialogueError};

use super::api::{PreviewConditionRequestId, PreviewInputRevision};
use super::events::PreviewEvent;
use super::state::PreviewState;

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq)]
pub struct PreviewOutput {
    events: Vec<PreviewEvent>,
    state: PreviewState,
}

impl PreviewOutput {
    pub(crate) fn new(events: Vec<PreviewEvent>, state: PreviewState) -> Self {
        Self { events, state }
    }

    #[must_use]
    pub fn events(&self) -> &[PreviewEvent] {
        &self.events
    }

    #[must_use]
    pub fn state(&self) -> &PreviewState {
        &self.state
    }
}

#[non_exhaustive]
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum PreviewError {
    #[error("preview runtime operation failed: {0}")]
    Runtime(#[from] DialogueError),
    #[error("preview asset revision could not be computed: {reason}")]
    AssetRevisionFailed { reason: String },
    #[error("preview is waiting for a condition answer")]
    ConditionPending,
    #[error("preview has no pending condition answer")]
    ConditionNotPending,
    #[error("condition request id {actual} does not match pending request {expected}")]
    ConditionRequestMismatch {
        expected: PreviewConditionRequestId,
        actual: PreviewConditionRequestId,
    },
    #[error("condition replay requires input revision {expected}, got {actual}")]
    InputRevisionMismatch {
        expected: PreviewInputRevision,
        actual: PreviewInputRevision,
    },
    #[error("condition `{function}` returned {actual} but preview expected {expected}")]
    ConditionResultTypeMismatch {
        function: String,
        expected: ConditionExpectedType,
        actual: ConditionExpectedType,
    },
    #[error("condition replay no longer matches the pending request: {mismatch}")]
    ConditionReplayMismatch { mismatch: String },
    #[error("condition request {request_id} failed: {reason}")]
    ConditionFailed {
        request_id: PreviewConditionRequestId,
        reason: String,
    },
    #[error("condition request id counter overflowed")]
    ConditionRequestIdOverflow,
    #[error("cannot snapshot while a condition answer is pending")]
    SnapshotPendingCondition,
    #[error("failed to encode preview snapshot: {reason}")]
    SnapshotEncodeFailed { reason: String },
    #[error("failed to decode preview snapshot: {reason}")]
    SnapshotDecodeFailed { reason: String },
    #[error("unsupported preview snapshot format {snapshot_format_version}")]
    UnsupportedSnapshotFormat { snapshot_format_version: u16 },
    #[error("preview snapshot belongs to asset `{actual:?}`, expected `{expected:?}`")]
    SnapshotAssetMismatch {
        expected: CompiledAssetId,
        actual: CompiledAssetId,
    },
    #[error("preview snapshot control projection does not match its runtime session")]
    SnapshotStateMismatch,
    #[error("restored blocking effect `{effect_id}` must be re-emitted before acknowledgement")]
    EffectRestorePending { effect_id: recite_core::EffectId },
}
