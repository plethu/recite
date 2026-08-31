use crate::preview_hash_dialogue::hash_availability;
use crate::preview_hash_primitives::{
    hash_expected_type, hash_len, hash_schema_fingerprint, hash_text, hash_u64, tag,
};

pub(super) fn hash_dialogue_error(
    hasher: &mut blake3::Hasher,
    error: &recite_runtime::DialogueError,
) {
    use recite_runtime::DialogueError;
    match error {
        DialogueError::UnknownBlock { block } => {
            tag(hasher, 0);
            hash_text(hasher, block);
        }
        DialogueError::UnsupportedCompiledFormat {
            format_version,
            compiler_compatibility_version,
        } => {
            tag(hasher, 1);
            hash_u64(hasher, *format_version as u64);
            hash_u64(hasher, *compiler_compatibility_version as u64);
        }
        DialogueError::AssetMismatch {
            expected_asset_id,
            actual_asset_id,
            expected_format_version,
            actual_format_version,
            expected_compiler_compatibility_version,
            actual_compiler_compatibility_version,
        } => {
            tag(hasher, 2);
            hash_text(hasher, expected_asset_id);
            hash_text(hasher, actual_asset_id);
            hash_u64(hasher, *expected_format_version as u64);
            hash_u64(hasher, *actual_format_version as u64);
            hash_u64(hasher, *expected_compiler_compatibility_version as u64);
            hash_u64(hasher, *actual_compiler_compatibility_version as u64);
        }
        DialogueError::AssetContentMismatch { asset_id, reason } => {
            tag(hasher, 3);
            hash_text(hasher, asset_id);
            hash_text(hasher, reason);
        }
        DialogueError::SchemaMismatch {
            asset_id,
            expected_schema_fingerprint,
            actual_schema_fingerprint,
        } => {
            tag(hasher, 4);
            hash_text(hasher, asset_id);
            hash_schema_fingerprint(hasher, expected_schema_fingerprint);
            hash_schema_fingerprint(hasher, actual_schema_fingerprint);
        }
        DialogueError::MalformedCompiledAsset { reason } => {
            tag(hasher, 5);
            hash_text(hasher, reason);
        }
        DialogueError::EffectPending { effect } | DialogueError::NoEffectPending { effect } => {
            tag(hasher, 6);
            hash_text(hasher, effect.as_str());
        }
        DialogueError::WrongEffectAcknowledgement { expected, actual } => {
            tag(hasher, 7);
            hash_text(hasher, expected.as_str());
            hash_text(hasher, actual.as_str());
        }
        DialogueError::PromptPending { choices } => {
            tag(hasher, 8);
            hash_ids(hasher, choices);
        }
        DialogueError::NoPromptPending { choice } => {
            tag(hasher, 9);
            hash_text(hasher, choice.as_str());
        }
        DialogueError::InvalidChoice {
            choice,
            prompt_choices,
        } => {
            tag(hasher, 10);
            hash_text(hasher, choice.as_str());
            hash_ids(hasher, prompt_choices);
        }
        DialogueError::UnavailableChoice {
            choice,
            availability,
        } => {
            tag(hasher, 11);
            hash_text(hasher, choice.as_str());
            hash_availability(hasher, availability);
        }
        DialogueError::ConditionEvaluationFailed { function, reason } => {
            tag(hasher, 12);
            hash_text(hasher, function);
            hash_text(hasher, reason);
        }
        DialogueError::ConditionResultTypeMismatch {
            function,
            expected,
            actual,
        } => {
            tag(hasher, 13);
            hash_text(hasher, function);
            hash_expected_type(hasher, *expected);
            hash_expected_type(hasher, *actual);
        }
        DialogueError::ConditionDepthLimitExceeded { limit } => {
            tag(hasher, 14);
            hash_len(hasher, *limit);
        }
        DialogueError::InterpolationValueFailed { name, reason } => {
            tag(hasher, 15);
            hash_text(hasher, name);
            hash_text(hasher, reason);
        }
        DialogueError::MissingInterpolationValue { name } => {
            tag(hasher, 16);
            hash_text(hasher, name);
        }
        DialogueError::InvalidInterpolationSyntax { reason } => {
            tag(hasher, 17);
            hash_text(hasher, reason);
        }
        DialogueError::LocaleLookupFailed { id, reason } => {
            tag(hasher, 18);
            hash_text(hasher, id);
            hash_text(hasher, reason);
        }
        DialogueError::InvalidPluralCount { name, reason } => {
            tag(hasher, 19);
            hash_text(hasher, name);
            hash_text(hasher, reason);
        }
        DialogueError::UnsupportedSessionSnapshotFormat {
            snapshot_format_version,
        } => {
            tag(hasher, 20);
            hash_u64(hasher, *snapshot_format_version as u64);
        }
        DialogueError::SessionSnapshotEncodeFailed { reason } => {
            tag(hasher, 21);
            hash_text(hasher, reason);
        }
        DialogueError::SessionSnapshotDecodeFailed { reason } => {
            tag(hasher, 22);
            hash_text(hasher, reason);
        }
        DialogueError::InvalidSessionSnapshot { reason, source } => {
            tag(hasher, 23);
            hash_text(hasher, reason);
            if let Some(source) = source {
                tag(hasher, 1);
                hash_snapshot_conversion_error(hasher, source);
            } else {
                tag(hasher, 0);
            }
        }
        DialogueError::SessionEnded => tag(hasher, 24),
        DialogueError::TraversalLimitExceeded { limit } => {
            tag(hasher, 25);
            hash_len(hasher, *limit);
        }
    }
}

fn hash_snapshot_conversion_error(
    hasher: &mut blake3::Hasher,
    error: &recite_runtime::DialogueSessionSnapshotConversionError,
) {
    match error {
        recite_runtime::DialogueSessionSnapshotConversionError::InvalidAvailabilityReasonId {
            id,
            source,
        } => {
            tag(hasher, 0);
            hash_text(hasher, id);
            match source {
                recite_core::CoreValueError::ZeroSourceLine => tag(hasher, 0),
                recite_core::CoreValueError::ZeroSourceColumn => tag(hasher, 1),
                recite_core::CoreValueError::EmptyDiagnosticCode => tag(hasher, 2),
                recite_core::CoreValueError::NonNamespacedDiagnosticCode(value) => {
                    tag(hasher, 3);
                    hash_text(hasher, value);
                }
                recite_core::CoreValueError::EmptyId { kind } => {
                    tag(hasher, 4);
                    hash_text(hasher, kind);
                }
                recite_core::CoreValueError::InvalidValue { kind, value } => {
                    tag(hasher, 5);
                    hash_text(hasher, kind);
                    hash_text(hasher, value);
                }
            }
        }
    }
}

fn hash_ids(hasher: &mut blake3::Hasher, ids: &[recite_core::ChoiceId]) {
    hash_len(hasher, ids.len());
    for id in ids {
        hash_text(hasher, id.as_str());
    }
}
