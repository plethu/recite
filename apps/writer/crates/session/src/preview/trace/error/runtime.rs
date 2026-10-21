use super::{Field, fingerprint, reason_fields};
use recite_runtime::DialogueError;

pub(super) fn fields(error: &DialogueError) -> Vec<Field<'_>> {
    match error {
        DialogueError::UnknownBlock { block } => vec![
            Field::text("Error", "Unknown beat"),
            Field::text("Beat", block.as_str()),
        ],
        DialogueError::UnsupportedCompiledFormat {
            format_version,
            compiler_compatibility_version,
        } => vec![
            Field::text("Error", "Unsupported compiled format"),
            Field::display("Format version", format_version),
            Field::display(
                "Compiler compatibility version",
                compiler_compatibility_version,
            ),
        ],
        DialogueError::AssetMismatch {
            expected_asset_id,
            actual_asset_id,
            expected_format_version,
            actual_format_version,
            expected_compiler_compatibility_version,
            actual_compiler_compatibility_version,
        } => vec![
            Field::text("Error", "Asset mismatch"),
            Field::text("Expected asset", expected_asset_id.as_str()),
            Field::text("Actual asset", actual_asset_id.as_str()),
            Field::display("Expected format version", expected_format_version),
            Field::display("Actual format version", actual_format_version),
            Field::display(
                "Expected compiler compatibility",
                expected_compiler_compatibility_version,
            ),
            Field::display(
                "Actual compiler compatibility",
                actual_compiler_compatibility_version,
            ),
        ],
        DialogueError::AssetContentMismatch { asset_id, reason } => vec![
            Field::text("Error", "Asset content mismatch"),
            Field::text("Asset", asset_id.as_str()),
            Field::text("Reason", reason.as_str()),
        ],
        DialogueError::SchemaMismatch {
            asset_id,
            expected_schema_fingerprint,
            actual_schema_fingerprint,
        } => {
            let mut fields = vec![
                Field::text("Error", "Schema fingerprint mismatch"),
                Field::text("Asset", asset_id.as_str()),
            ];
            fields.extend(fingerprint::fields(
                expected_schema_fingerprint,
                actual_schema_fingerprint,
            ));
            fields
        }
        DialogueError::MalformedCompiledAsset { reason } => {
            reason_fields("Malformed compiled asset", reason)
        }
        DialogueError::EffectPending { effect } => vec![
            Field::text("Error", "Effect pending"),
            Field::text("Effect ID", effect.as_str()),
        ],
        DialogueError::NoEffectPending { effect } => vec![
            Field::text("Error", "No effect pending"),
            Field::text("Effect ID", effect.as_str()),
        ],
        DialogueError::WrongEffectAcknowledgement { expected, actual } => vec![
            Field::text("Error", "Wrong effect acknowledgement"),
            Field::text("Expected effect ID", expected.as_str()),
            Field::text("Actual effect ID", actual.as_str()),
        ],
        DialogueError::PromptPending { choices } => vec![
            Field::text("Error", "Choice pending"),
            Field::display("Offered choices", choices.len()),
        ],
        DialogueError::NoPromptPending { choice } => vec![
            Field::text("Error", "No choice pending"),
            Field::text("Choice ID", choice.as_str()),
        ],
        DialogueError::InvalidChoice {
            choice,
            prompt_choices,
        } => vec![
            Field::text("Error", "Invalid choice"),
            Field::text("Choice ID", choice.as_str()),
            Field::display("Offered choices", prompt_choices.len()),
        ],
        DialogueError::UnavailableChoice {
            choice,
            availability,
        } => vec![
            Field::text("Error", "Unavailable choice"),
            Field::text("Choice ID", choice.as_str()),
            Field::optional(
                "Reason",
                availability
                    .primary_reason
                    .as_ref()
                    .map(|reason| reason.text.as_str()),
            ),
        ],
        DialogueError::ConditionEvaluationFailed {
            function, reason, ..
        } => named_reason("Condition evaluation failed", "Function", function, reason),
        DialogueError::ConditionResultTypeMismatch {
            function,
            expected,
            actual,
        } => vec![
            Field::text("Error", "Condition result type mismatch"),
            Field::text("Function", function.as_str()),
            Field::display("Expected type", expected),
            Field::display("Actual type", actual),
        ],
        DialogueError::ConditionDepthLimitExceeded { limit } => vec![
            Field::text("Error", "Condition depth exceeded"),
            Field::display("Depth limit", limit),
        ],
        DialogueError::InterpolationValueFailed { name, reason } => {
            named_reason("Interpolation failed", "Value name", name, reason)
        }
        DialogueError::MissingInterpolationValue { name } => vec![
            Field::text("Error", "Missing interpolation value"),
            Field::text("Value name", name.as_str()),
        ],
        DialogueError::InvalidInterpolationSyntax { reason } => {
            reason_fields("Invalid interpolation syntax", reason)
        }
        DialogueError::LocaleLookupFailed { id, reason } => {
            named_reason("Locale lookup failed", "Stable ID", id, reason)
        }
        DialogueError::InvalidPluralCount { name, reason } => {
            named_reason("Invalid plural count", "Value name", name, reason)
        }
        DialogueError::UnsupportedSessionSnapshotFormat {
            snapshot_format_version,
        } => vec![
            Field::text("Error", "Unsupported session snapshot format"),
            Field::display("Format version", snapshot_format_version),
        ],
        DialogueError::SessionSnapshotEncodeFailed { reason } => {
            reason_fields("Session snapshot encoding failed", reason)
        }
        DialogueError::SessionSnapshotDecodeFailed { reason } => {
            reason_fields("Session snapshot decoding failed", reason)
        }
        DialogueError::InvalidSessionSnapshot { reason, .. } => {
            reason_fields("Invalid session snapshot", reason)
        }
        DialogueError::SessionEnded => {
            reason_fields("Session ended", "The conversation has already ended")
        }
        DialogueError::TraversalLimitExceeded { limit } => vec![
            Field::text("Error", "Traversal limit exceeded"),
            Field::display("Step limit", limit),
        ],
    }
}
fn named_reason<'a>(
    category: &'static str,
    name: &'static str,
    value: &'a str,
    reason: &'a str,
) -> Vec<Field<'a>> {
    vec![
        Field::text("Error", category),
        Field::text(name, value),
        Field::text("Reason", reason),
    ]
}
