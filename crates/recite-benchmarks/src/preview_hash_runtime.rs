use recite_runtime::{
    ConditionValue, DialogueEffectArgument, DialogueEffectRequest, PreviewConditionArgument,
    PreviewConditionResult,
};

use crate::preview_hash_dialogue::hash_optional_prompt;
use crate::preview_hash_primitives::{
    hash_bool, hash_bytes, hash_expected_type, hash_i64, hash_len, hash_text, hash_u64, tag,
};

pub(super) fn hash_request(
    hasher: &mut blake3::Hasher,
    request: &recite_runtime::PreviewConditionRequest,
) {
    hash_u64(hasher, request.id().get());
    hash_text(hasher, request.block().as_str());
    hash_optional_prompt(hasher, request.prompt());
    hash_query(hasher, request.query());
}

pub(super) fn hash_query(
    hasher: &mut blake3::Hasher,
    query: &recite_runtime::PreviewConditionQuery,
) {
    hash_text(hasher, query.function());
    hash_expected_type(hasher, query.expected_type());
    hash_len(hasher, query.arguments().len());
    for argument in query.arguments() {
        match argument {
            PreviewConditionArgument::Identifier(value) => {
                tag(hasher, 0);
                hash_text(hasher, value);
            }
            PreviewConditionArgument::String(value) => {
                tag(hasher, 1);
                hash_text(hasher, value);
            }
            PreviewConditionArgument::Integer(value) => {
                tag(hasher, 2);
                hash_i64(hasher, *value);
            }
            PreviewConditionArgument::Float(value) => {
                tag(hasher, 3);
                hash_u64(hasher, value.to_bits());
            }
            PreviewConditionArgument::Boolean(value) => {
                tag(hasher, 4);
                hash_bool(hasher, *value);
            }
            _ => tag(hasher, 255),
        }
    }
}

pub(super) fn hash_result(hasher: &mut blake3::Hasher, result: &PreviewConditionResult) {
    match result {
        PreviewConditionResult::Value(ConditionValue::Bool(value)) => {
            tag(hasher, 0);
            hash_bool(hasher, *value);
        }
        PreviewConditionResult::Value(ConditionValue::EnumVariant(value)) => {
            tag(hasher, 1);
            hash_text(hasher, value);
        }
        PreviewConditionResult::Failed { reason } => {
            tag(hasher, 2);
            hash_text(hasher, reason);
        }
        _ => tag(hasher, 255),
    }
}

pub(super) fn hash_effect(hasher: &mut blake3::Hasher, effect: &DialogueEffectRequest) {
    hash_text(hasher, effect.id.as_str());
    tag(
        hasher,
        match effect.mode {
            recite_runtime::DialogueEffectMode::Deferred => 0,
            recite_runtime::DialogueEffectMode::Immediate => 1,
            recite_runtime::DialogueEffectMode::Blocking => 2,
        },
    );
    hash_text(hasher, &effect.function);
    hash_len(hasher, effect.args.len());
    for argument in &effect.args {
        match argument {
            DialogueEffectArgument::Identifier(value) => {
                tag(hasher, 0);
                hash_text(hasher, value);
            }
            DialogueEffectArgument::String(value) => {
                tag(hasher, 1);
                hash_text(hasher, value);
            }
            DialogueEffectArgument::Integer(value) => {
                tag(hasher, 2);
                hash_i64(hasher, *value);
            }
            DialogueEffectArgument::Float(value) => {
                tag(hasher, 3);
                hash_u64(hasher, value.to_bits());
            }
            DialogueEffectArgument::Boolean(value) => {
                tag(hasher, 4);
                hash_bool(hasher, *value);
            }
        }
    }
    crate::preview_hash_primitives::hash_span(hasher, &effect.source_span);
}

pub(super) fn hash_revision(
    hasher: &mut blake3::Hasher,
    revision: &recite_runtime::PreviewAssetRevision,
) {
    hash_text(hasher, revision.asset_id().as_str());
    hash_text(hasher, revision.payload_fingerprint().algorithm().as_str());
    hash_bytes(hasher, revision.payload_fingerprint().digest().as_bytes());
}
