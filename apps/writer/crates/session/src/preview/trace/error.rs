//! Borrow error strings directly; formatting a complete error can copy arbitrary caller content.
use super::Field;
use recite_runtime::preview::PreviewError;
mod fingerprint;
mod runtime;

pub(super) fn fields(error: &PreviewError) -> Vec<Field<'_>> {
    match error {
        PreviewError::Runtime(error) => runtime::fields(error),
        PreviewError::AssetRevisionFailed { reason } => {
            reason_fields("Asset revision failed", reason)
        }
        PreviewError::ConditionPending => {
            reason_fields("Condition pending", "Waiting for a condition answer")
        }
        PreviewError::ConditionNotPending => {
            reason_fields("No condition pending", "There is no condition to answer")
        }
        PreviewError::ConditionRequestMismatch { expected, actual } => vec![
            Field::text("Error", "Condition request mismatch"),
            Field::display("Expected request", expected),
            Field::display("Actual request", actual),
        ],
        PreviewError::InputRevisionMismatch { expected, actual } => vec![
            Field::text("Error", "Input revision mismatch"),
            Field::display("Expected revision", expected),
            Field::display("Actual revision", actual),
        ],
        PreviewError::ConditionResultTypeMismatch {
            function,
            expected,
            actual,
        } => vec![
            Field::text("Error", "Condition result type mismatch"),
            Field::text("Function", function.as_str()),
            Field::display("Expected type", expected),
            Field::display("Actual type", actual),
        ],
        PreviewError::ConditionReplayMismatch { mismatch } => {
            reason_fields("Condition replay mismatch", mismatch)
        }
        PreviewError::ConditionFailed { request_id, reason } => vec![
            Field::text("Error", "Condition failed"),
            Field::display("Condition request", request_id),
            Field::text("Reason", reason.as_str()),
        ],
        PreviewError::ConditionRequestIdOverflow => reason_fields(
            "Condition request overflow",
            "The condition request counter is exhausted",
        ),
        PreviewError::SnapshotPendingCondition => {
            reason_fields("Snapshot unavailable", "Answer the pending condition first")
        }
        PreviewError::SnapshotEncodeFailed { reason } => {
            reason_fields("Snapshot encoding failed", reason)
        }
        PreviewError::SnapshotDecodeFailed { reason } => {
            reason_fields("Snapshot decoding failed", reason)
        }
        PreviewError::UnsupportedSnapshotFormat {
            snapshot_format_version,
        } => vec![
            Field::text("Error", "Unsupported snapshot format"),
            Field::display("Format version", snapshot_format_version),
        ],
        PreviewError::SnapshotAssetMismatch { expected, actual } => vec![
            Field::text("Error", "Snapshot asset mismatch"),
            Field::text("Expected asset", expected.as_str()),
            Field::text("Actual asset", actual.as_str()),
        ],
        PreviewError::SnapshotStateMismatch => reason_fields(
            "Snapshot state mismatch",
            "The snapshot projection differs from its runtime session",
        ),
        PreviewError::EffectRestorePending { effect_id } => vec![
            Field::text("Error", "Effect restore pending"),
            Field::text("Effect ID", effect_id.as_str()),
        ],
        _ => vec![Field::text("Error", "Unsupported preview error variant")],
    }
}
fn reason_fields<'a>(category: &'static str, reason: &'a str) -> Vec<Field<'a>> {
    vec![
        Field::text("Error", category),
        Field::text("Reason", reason),
    ]
}
