use crate::preview_hash_dialogue_errors::hash_dialogue_error;
use crate::preview_hash_primitives::{hash_expected_type, hash_text, hash_u64, tag};

pub(super) fn hash_preview_error(
    hasher: &mut blake3::Hasher,
    error: &recite_runtime::PreviewError,
) {
    use recite_runtime::PreviewError;
    match error {
        PreviewError::Runtime(error) => {
            tag(hasher, 0);
            hash_dialogue_error(hasher, error);
        }
        PreviewError::AssetRevisionFailed { reason } => {
            tag(hasher, 1);
            hash_text(hasher, reason);
        }
        PreviewError::ConditionPending => tag(hasher, 2),
        PreviewError::ConditionNotPending => tag(hasher, 3),
        PreviewError::ConditionRequestMismatch { expected, actual } => {
            tag(hasher, 4);
            hash_u64(hasher, expected.get());
            hash_u64(hasher, actual.get());
        }
        PreviewError::InputRevisionMismatch { expected, actual } => {
            tag(hasher, 5);
            hash_u64(hasher, expected.get());
            hash_u64(hasher, actual.get());
        }
        PreviewError::ConditionResultTypeMismatch {
            function,
            expected,
            actual,
        } => {
            tag(hasher, 6);
            hash_text(hasher, function);
            hash_expected_type(hasher, *expected);
            hash_expected_type(hasher, *actual);
        }
        PreviewError::ConditionReplayMismatch { mismatch } => {
            tag(hasher, 7);
            hash_text(hasher, mismatch);
        }
        PreviewError::ConditionFailed { request_id, reason } => {
            tag(hasher, 8);
            hash_u64(hasher, request_id.get());
            hash_text(hasher, reason);
        }
        PreviewError::ConditionRequestIdOverflow => tag(hasher, 9),
        PreviewError::SnapshotPendingCondition => tag(hasher, 10),
        PreviewError::SnapshotEncodeFailed { reason } => {
            tag(hasher, 11);
            hash_text(hasher, reason);
        }
        PreviewError::SnapshotDecodeFailed { reason } => {
            tag(hasher, 12);
            hash_text(hasher, reason);
        }
        PreviewError::UnsupportedSnapshotFormat {
            snapshot_format_version,
        } => {
            tag(hasher, 13);
            hash_u64(hasher, *snapshot_format_version as u64);
        }
        PreviewError::SnapshotAssetMismatch { expected, actual } => {
            tag(hasher, 14);
            hash_text(hasher, expected.as_str());
            hash_text(hasher, actual.as_str());
        }
        PreviewError::SnapshotStateMismatch => tag(hasher, 15),
        PreviewError::EffectRestorePending { effect_id } => {
            tag(hasher, 16);
            hash_text(hasher, effect_id.as_str());
        }
        _ => tag(hasher, 255),
    }
}
