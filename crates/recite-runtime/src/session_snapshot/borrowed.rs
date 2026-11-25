//! Encode borrowed session rows without materializing an owned save snapshot.
use recite_core::{
    ChoiceId,
    compiled::{CompiledDialogue, CompiledSourceFile},
};
use serde::{Serialize, Serializer};

use super::{
    CURRENT_SESSION_SNAPSHOT_FORMAT_VERSION, DialogueBlockDigestsSnapshot,
    DialogueContentFingerprintSnapshot, DialogueSchemaFingerprintSnapshot,
    DialogueSessionPendingPromptSnapshot, DialogueSessionRangeSnapshot, EffectRequestRef,
    content_fingerprint_snapshot, frame_snapshot, pending_prompt_snapshot, range_snapshot,
    schema_fingerprint_snapshot,
};
use crate::resume::{BlockDigests, ResumeAnchor, block_digests, resume_anchor};
use crate::traversal::AssetView;
use crate::{
    DialogueEffectRequest, DialogueError, DialogueSession,
    session::{PendingEffect, SessionPhase, StatementFrame},
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
    block_id: &'a str,
    #[serde(serialize_with = "digests")]
    block_digests: BlockDigests,
    current_block: u32,
    current_range: DialogueSessionRangeSnapshot,
    next_statement: u32,
    #[serde(serialize_with = "frames")]
    continuation_stack: &'a [StatementFrame],
    resume_anchor: ResumeAnchor<'a>,
    pending_prompt: Option<DialogueSessionPendingPromptSnapshot>,
    pending_effect: Option<PendingEffectRef<'a>>,
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
    /// Projects a session that runs against `asset`. Stable IDs and digests
    /// come from the asset, so it must be the session's own asset.
    pub(crate) fn new(
        asset: &'a CompiledDialogue,
        session: &'a DialogueSession,
    ) -> Result<Self, DialogueError> {
        let view = AssetView::new(asset)?;
        view.ensure_session_matches(session)?;
        let block = view.block_at(session.current_block)?;
        Ok(Self {
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
            block_id: block.id.as_str(),
            block_digests: block_digests(view, block)?,
            current_block: session.current_block.as_u32(),
            current_range: range_snapshot(session.current_range),
            next_statement: session.next_statement.as_u32(),
            continuation_stack: &session.continuation_stack,
            resume_anchor: resume_anchor(
                view,
                &session.continuation_stack,
                session.current_range,
                session.next_statement,
            )?,
            pending_prompt: match &session.phase {
                SessionPhase::AwaitingChoice(prompt) => Some(pending_prompt_snapshot(prompt)),
                _ => None,
            },
            pending_effect: match &session.phase {
                SessionPhase::AwaitingEffect(effect) => Some(PendingEffectRef::new(effect)),
                _ => None,
            },
            previous_prompt_choices: &session.previous_prompt_choices,
            selected_choice_history: &session.selected_choice_history,
            deferred_effects: &session.deferred_effects,
            locale: session.locale.as_ref().map(|locale| locale.as_str()),
            trace_counter: session.trace_counter,
            ended: matches!(session.phase, SessionPhase::Ended),
        })
    }

    pub(crate) fn block_id(&self) -> &'a str {
        self.block_id
    }

    pub(crate) fn block_digests(&self) -> DialogueBlockDigestsSnapshot {
        self.block_digests.to_snapshot()
    }

    pub(crate) fn resume_anchor(&self) -> &ResumeAnchor<'a> {
        &self.resume_anchor
    }
}

#[derive(Serialize)]
struct PendingEffectRef<'a> {
    statement: u32,
    request: EffectRequestRef<'a>,
}

impl<'a> PendingEffectRef<'a> {
    fn new(effect: &'a PendingEffect) -> Self {
        Self {
            statement: effect.statement.as_u32(),
            request: EffectRequestRef::new(&effect.request),
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

#[derive(Serialize)]
struct BlockDigestsRef<'a> {
    content: FingerprintRef<'a>,
    structure: FingerprintRef<'a>,
}

fn digests<S: Serializer>(digests: &BlockDigests, serializer: S) -> Result<S::Ok, S::Error> {
    BlockDigestsRef {
        content: FingerprintRef {
            algorithm: crate::resume::DIGEST_ALGORITHM,
            digest: &digests.content,
        },
        structure: FingerprintRef {
            algorithm: crate::resume::DIGEST_ALGORITHM,
            digest: &digests.structure,
        },
    }
    .serialize(serializer)
}

fn frames<S: Serializer>(frames: &[StatementFrame], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(frames.iter().map(frame_snapshot))
}

fn choices<S: Serializer>(choices: &[ChoiceId], serializer: S) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(choices.iter().map(ChoiceId::as_str))
}

fn effects<S: Serializer>(
    effects: &[DialogueEffectRequest],
    serializer: S,
) -> Result<S::Ok, S::Error> {
    serializer.collect_seq(effects.iter().map(EffectRequestRef::new))
}
