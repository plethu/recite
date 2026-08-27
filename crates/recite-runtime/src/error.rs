use recite_core::{ChoiceId, EffectId};

use crate::session_snapshot::DialogueSessionSnapshotConversionError;
use crate::{
    ChoiceAvailability, ConditionExpectedType, DialogueEffectMode,
    DialogueSchemaFingerprintSnapshot,
};

/// Runtime error for deterministic traversal over compiled dialogue assets.
#[derive(Clone, Debug, PartialEq, thiserror::Error)]
pub enum DialogueError {
    #[error("unknown block `{block}`")]
    UnknownBlock { block: String },
    #[error(
        "unsupported compiled asset format {format_version} with compatibility version {compiler_compatibility_version}"
    )]
    UnsupportedCompiledFormat {
        format_version: u16,
        compiler_compatibility_version: u16,
    },
    #[error(
        "session is for asset `{expected_asset_id}` ({expected_format_version}/{expected_compiler_compatibility_version}) but got `{actual_asset_id}` ({actual_format_version}/{actual_compiler_compatibility_version})"
    )]
    AssetMismatch {
        expected_asset_id: String,
        actual_asset_id: String,
        expected_format_version: u16,
        actual_format_version: u16,
        expected_compiler_compatibility_version: u16,
        actual_compiler_compatibility_version: u16,
    },
    #[error("session is for a different compiled asset payload `{asset_id}`: {reason}")]
    AssetContentMismatch { asset_id: String, reason: String },
    #[error(
        "session for asset `{asset_id}` has schema fingerprint {expected_schema_fingerprint:?}, but the provided asset has {actual_schema_fingerprint:?}"
    )]
    SchemaMismatch {
        asset_id: String,
        expected_schema_fingerprint: DialogueSchemaFingerprintSnapshot,
        actual_schema_fingerprint: DialogueSchemaFingerprintSnapshot,
    },
    #[error("malformed compiled asset: {reason}")]
    MalformedCompiledAsset { reason: String },
    #[error("session is waiting for effect `{effect}` to be acknowledged")]
    EffectPending { effect: EffectId },
    #[error("effect `{effect}` was acknowledged with no pending effect")]
    NoEffectPending { effect: EffectId },
    #[error("effect acknowledgement `{actual}` does not match pending effect `{expected}`")]
    WrongEffectAcknowledgement {
        expected: EffectId,
        actual: EffectId,
    },
    #[error("session is waiting for a choice selection")]
    PromptPending { choices: Vec<ChoiceId> },
    #[error("choice `{choice}` was selected with no pending prompt")]
    NoPromptPending { choice: ChoiceId },
    #[error("choice `{choice}` is not available in the pending prompt")]
    InvalidChoice {
        choice: ChoiceId,
        prompt_choices: Vec<ChoiceId>,
    },
    #[error(fmt = fmt_unavailable_choice)]
    UnavailableChoice {
        choice: ChoiceId,
        availability: Box<ChoiceAvailability>,
    },
    #[error("condition `{function}` failed: {reason}")]
    ConditionEvaluationFailed { function: String, reason: String },
    #[error("condition `{function}` returned {actual} but runtime expected {expected}")]
    ConditionResultTypeMismatch {
        function: String,
        expected: ConditionExpectedType,
        actual: ConditionExpectedType,
    },
    #[error("condition expression exceeded maximum evaluation depth {limit}")]
    ConditionDepthLimitExceeded { limit: usize },
    #[error("unsupported session snapshot format {snapshot_format_version}")]
    UnsupportedSessionSnapshotFormat { snapshot_format_version: u16 },
    #[error("failed to encode session snapshot: {reason}")]
    SessionSnapshotEncodeFailed { reason: String },
    #[error("failed to decode session snapshot: {reason}")]
    SessionSnapshotDecodeFailed { reason: String },
    #[error("invalid session snapshot: {reason}")]
    InvalidSessionSnapshot {
        reason: String,
        #[source]
        source: Option<Box<DialogueSessionSnapshotConversionError>>,
    },
    #[error("session has already ended")]
    SessionEnded,
    #[error("runtime traversal exceeded {limit} internal steps")]
    TraversalLimitExceeded { limit: usize },
}

fn fmt_unavailable_choice(
    choice: &ChoiceId,
    availability: &ChoiceAvailability,
    formatter: &mut std::fmt::Formatter<'_>,
) -> std::fmt::Result {
    write!(formatter, "choice `{choice}` is unavailable")?;
    if let Some(reason) = &availability.primary_reason {
        write!(formatter, ": {}", reason.text)?;
    }
    Ok(())
}

impl std::fmt::Display for ConditionExpectedType {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Bool => formatter.write_str("bool"),
            Self::Enum => formatter.write_str("enum"),
        }
    }
}

impl std::fmt::Display for DialogueEffectMode {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Deferred => formatter.write_str("deferred"),
            Self::Immediate => formatter.write_str("immediate"),
            Self::Blocking => formatter.write_str("blocking"),
        }
    }
}
