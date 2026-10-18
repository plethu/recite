//! Encode borrowed session rows without materializing an owned save snapshot.
use recite_core::{ChoiceId, compiled::CompiledSourceFile};
use serde::{Serialize, Serializer};

use super::{
    CURRENT_SESSION_SNAPSHOT_FORMAT_VERSION, DialogueContentFingerprintSnapshot,
    DialogueSchemaFingerprintSnapshot, DialogueSessionPendingEffectSnapshot,
    DialogueSessionPendingPromptSnapshot, DialogueSessionRangeSnapshot,
    content_fingerprint_snapshot, frame_snapshot, pending_effect_snapshot, pending_prompt_snapshot,
    range_snapshot, schema_fingerprint_snapshot,
};
use crate::{
    DialogueEffectRequest, DialogueSession,
    session::{SessionPhase, StatementFrame},
};

// Field order follows DialogueSessionSnapshot. Exact-byte differential tests
// keep this private borrowing projection aligned with the public owned snapshot.
#[derive(Serialize)]
pub(crate) struct SessionSnapshotRef<'a> {
    snapshot_format_version: u16,
    asset_id: &'a str,
    asset_format_version: u16,
    asset_compiler_compatibility_version: u16,
    compiler_version: &'a str,
    source_map_id: &'a str,
    compiled_payload_fingerprint: DialogueContentFingerprintSnapshot,
    schema_fingerprint: DialogueSchemaFingerprintSnapshot,
    #[serde(serialize_with = "sources")]
    sources: &'a [CompiledSourceFile],
    current_block: u32,
    current_range: DialogueSessionRangeSnapshot,
    next_statement: u32,
    #[serde(serialize_with = "frames")]
    continuation_stack: &'a [StatementFrame],
    pending_prompt: Option<DialogueSessionPendingPromptSnapshot>,
    pending_effect: Option<DialogueSessionPendingEffectSnapshot>,
    #[serde(serialize_with = "choices")]
    previous_prompt_choices: &'a [ChoiceId],
    #[serde(serialize_with = "choices")]
    selected_choice_history: &'a [ChoiceId],
    #[serde(serialize_with = "effects")]
    deferred_effects: &'a [DialogueEffectRequest],
    locale: Option<&'a str>,
    trace_counter: u64,
    ended: bool,
}

impl<'a> SessionSnapshotRef<'a> {
    pub(crate) fn new(session: &'a DialogueSession) -> Self {
        Self {
            snapshot_format_version: CURRENT_SESSION_SNAPSHOT_FORMAT_VERSION,
            asset_id: session.asset_id.as_str(),
            asset_format_version: session.format_version,
            asset_compiler_compatibility_version: session.compiler_compatibility_version,
            compiler_version: session.compiler_version.as_str(),
            source_map_id: session.source_map_id.as_str(),
            compiled_payload_fingerprint: content_fingerprint_snapshot(
                &session.compiled_payload_fingerprint,
            ),
            schema_fingerprint: schema_fingerprint_snapshot(&session.schema_fingerprint),
            sources: &session.sources,
            current_block: session.current_block.as_u32(),
            current_range: range_snapshot(session.current_range),
            next_statement: session.next_statement.as_u32(),
            continuation_stack: &session.continuation_stack,
            pending_prompt: match &session.phase {
                SessionPhase::AwaitingChoice(prompt) => Some(pending_prompt_snapshot(prompt)),
                _ => None,
            },
            pending_effect: match &session.phase {
                SessionPhase::AwaitingEffect(effect) => Some(pending_effect_snapshot(effect)),
                _ => None,
            },
            previous_prompt_choices: &session.previous_prompt_choices,
            selected_choice_history: &session.selected_choice_history,
            deferred_effects: &session.deferred_effects,
            locale: session.locale.as_ref().map(|locale| locale.as_str()),
            trace_counter: session.trace_counter,
            ended: matches!(session.phase, SessionPhase::Ended),
        }
    }
}

#[derive(Serialize)]
struct SourceRef<'a> {
    path: &'a str,
    fingerprint: FingerprintRef<'a>,
}

#[derive(Serialize)]
struct FingerprintRef<'a> {
    algorithm: &'a str,
    digest: &'a [u8],
}

fn sources<S: Serializer>(
    sources: &[CompiledSourceFile],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(sources.iter().map(|source| SourceRef {
        path: &source.path,
        fingerprint: FingerprintRef {
            algorithm: source.fingerprint.algorithm().as_str(),
            digest: source.fingerprint.digest().as_bytes(),
        },
    }))
}

fn frames<S: Serializer>(frames: &[StatementFrame], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(frames.iter().map(frame_snapshot))
}

fn choices<S: Serializer>(choices: &[ChoiceId], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(choices.iter().map(ChoiceId::as_str))
}

#[derive(Serialize)]
struct EffectRef<'a> {
    id: &'a str,
}

fn effects<S: Serializer>(
    effects: &[DialogueEffectRequest],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(effects.iter().map(|effect| EffectRef {
        id: effect.id.as_str(),
    }))
}
