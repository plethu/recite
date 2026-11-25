//! Resume a snapshot in the exact asset or relocate it in a patched one.

use std::sync::Arc;

use recite_core::{
    BlockId, ChoiceId,
    compiled::{BlockIndex, CompiledDialogue},
};

use crate::session_serialization::references::{
    choice_id, invalid_snapshot, invalid_snapshot_with_source, restore_locale,
};
use crate::session_serialization::{decode_session_snapshot_messagepack, restore_session};
use crate::session_snapshot::{
    CURRENT_SESSION_SNAPSHOT_FORMAT_VERSION, DialogueBlockDigestsSnapshot,
    DialogueLandmarkSnapshot, DialogueSessionSnapshot, content_fingerprint_snapshot,
    effect_request_from_snapshot, schema_fingerprint_snapshot,
};
use crate::traversal::AssetView;
use crate::{DialogueEffectRequest, DialogueError, DialogueSession};

use super::digest::{BlockDigests, block_digests};
use super::locate::locate_session;
use super::outcome::{
    PromptResume, ResumeFallback, ResumeFallbackReason, ResumeLandmark, ResumeLevel, ResumeOptions,
    ResumeReport, SessionResume,
};

/// Decodes MessagePack save bytes and resumes them against `asset`.
///
/// See [`resume_session`]. Undecodable bytes and unsupported snapshot formats
/// are errors, not fallbacks.
pub fn resume_session_messagepack(
    asset: &CompiledDialogue,
    bytes: &[u8],
    options: &ResumeOptions,
) -> Result<SessionResume, DialogueError> {
    resume_session(asset, decode_session_snapshot_messagepack(bytes)?, options)
}

/// Resumes a saved session against `asset`, which may be a patched revision
/// of the asset the session was saved against.
///
/// The result is a deterministic function of the snapshot, the asset and the
/// options. An identical asset restores exactly, with the same validation as
/// [`crate::snapshot::restore_session`]. A patched asset resumes when the saved
/// block still exists and the saved position relocates through its stable
/// IDs; otherwise the result is a [`ResumeFallback`] carrying the saved
/// effects. Errors are reserved for unsupported or malformed snapshots and for
/// an asset with a different asset ID.
pub fn resume_session(
    asset: &CompiledDialogue,
    snapshot: DialogueSessionSnapshot,
    options: &ResumeOptions,
) -> Result<SessionResume, DialogueError> {
    if snapshot.snapshot_format_version != CURRENT_SESSION_SNAPSHOT_FORMAT_VERSION {
        return Err(DialogueError::UnsupportedSessionSnapshotFormat {
            snapshot_format_version: snapshot.snapshot_format_version,
        });
    }
    let view = AssetView::new(asset)?;
    let header = &asset.header;
    if snapshot.asset_id != header.asset_id.as_str() {
        return Err(DialogueError::AssetMismatch {
            expected_asset_id: snapshot.asset_id,
            actual_asset_id: header.asset_id.as_str().to_owned(),
            expected_format_version: snapshot.asset_format_version,
            actual_format_version: header.format_version,
            expected_compiler_compatibility_version: snapshot.asset_compiler_compatibility_version,
            actual_compiler_compatibility_version: header.compiler_compatibility_version,
        });
    }
    let payload =
        asset
            .content_fingerprint()
            .map_err(|error| DialogueError::MalformedCompiledAsset {
                reason: error.to_string(),
            })?;
    let block_id = BlockId::new(snapshot.block_id.clone())
        .map_err(|error| invalid_snapshot(format!("saved block id: {error}")))?;
    if snapshot.compiled_payload_fingerprint == content_fingerprint_snapshot(payload) {
        let prompt = if snapshot.pending_prompt.is_some() {
            PromptResume::Kept
        } else {
            PromptResume::None
        };
        let session = restore_session(asset, snapshot)?;
        return Ok(SessionResume::Resumed {
            session,
            report: ResumeReport {
                level: ResumeLevel::Exact,
                block_id,
                prompt,
                schema_changed: false,
                compiler_changed: false,
            },
        });
    }

    let saved = SavedState::parse(&snapshot, block_id)?;
    let Some(block_index) = find_block(view, &saved.block_id)? else {
        return Ok(saved.fallback(ResumeFallbackReason::BlockRemoved, false));
    };
    let block = view.block_at(block_index)?;
    let digests = block_digests(view, block)?;
    let position = match locate_session(view, block, &snapshot)? {
        Ok(position) => position,
        Err(reason) => return Ok(saved.fallback(reason, true)),
    };
    let mut level = change_level(&snapshot.block_digests, digests);
    if position.prompt == PromptResume::Reevaluated {
        level = level.max(ResumeLevel::Relocated);
    }
    if level > options.max_level() {
        return Ok(saved.fallback(
            ResumeFallbackReason::LevelRefused {
                required: level,
                allowed: options.max_level(),
            },
            true,
        ));
    }

    let report = ResumeReport {
        level,
        block_id: saved.block_id.clone(),
        prompt: position.prompt,
        schema_changed: snapshot.schema_fingerprint
            != schema_fingerprint_snapshot(&header.schema_fingerprint),
        compiler_changed: snapshot.compiler_version != header.compiler_version.as_str(),
    };
    let session = DialogueSession {
        asset_id: header.asset_id.clone(),
        format_version: header.format_version,
        compiler_compatibility_version: header.compiler_compatibility_version,
        compiler_version: header.compiler_version.clone(),
        source_map_id: header.source_map_id.clone(),
        schema_fingerprint: header.schema_fingerprint.clone(),
        compiled_payload_fingerprint: payload.clone(),
        sources: asset.sources.as_slice().into(),
        current_block: block_index,
        current_range: position.current_range,
        next_statement: position.next_statement,
        continuation_stack: position.frames,
        phase: position.phase,
        previous_prompt_choices: saved.previous_prompt_choices,
        selected_choice_history: saved.selected_choice_history,
        deferred_effects: Arc::new(saved.deferred_effects),
        locale: saved.locale,
        trace_counter: snapshot.trace_counter,
    };
    Ok(SessionResume::Resumed { session, report })
}

/// Saved state that survives any patch.
struct SavedState {
    block_id: BlockId,
    landmark: Option<ResumeLandmark>,
    ended: bool,
    previous_prompt_choices: Vec<ChoiceId>,
    selected_choice_history: Vec<ChoiceId>,
    deferred_effects: Vec<DialogueEffectRequest>,
    pending_effect: Option<DialogueEffectRequest>,
    locale: Option<recite_core::LocaleId>,
}

impl SavedState {
    fn parse(snapshot: &DialogueSessionSnapshot, block_id: BlockId) -> Result<Self, DialogueError> {
        if snapshot.pending_prompt.is_some() && snapshot.pending_effect.is_some()
            || snapshot.ended
                && (snapshot.pending_prompt.is_some() || snapshot.pending_effect.is_some())
        {
            return Err(invalid_snapshot("session phase fields conflict"));
        }
        let anchor = &snapshot.resume_anchor;
        let landmark = std::iter::once(&anchor.position)
            .chain(anchor.frames.iter().rev().map(|frame| &frame.position))
            .find_map(|position| position.landmark.as_ref())
            .map(landmark_from_snapshot)
            .transpose()?;
        Ok(Self {
            block_id,
            landmark,
            ended: snapshot.ended,
            previous_prompt_choices: choice_ids(&snapshot.previous_prompt_choices)?,
            selected_choice_history: choice_ids(&snapshot.selected_choice_history)?,
            deferred_effects: snapshot
                .deferred_effects
                .iter()
                .map(|effect| {
                    effect_request_from_snapshot(effect).map_err(invalid_snapshot_with_source)
                })
                .collect::<Result<_, _>>()?,
            pending_effect: snapshot
                .pending_effect
                .as_ref()
                .map(|effect| {
                    effect_request_from_snapshot(&effect.request)
                        .map_err(invalid_snapshot_with_source)
                })
                .transpose()?,
            locale: restore_locale(snapshot.locale.as_deref())?,
        })
    }

    fn fallback(self, reason: ResumeFallbackReason, block_available: bool) -> SessionResume {
        // An ended session keeps the deferred effects it delivered with its end.
        let deferred_effects = if self.ended {
            Vec::new()
        } else {
            self.deferred_effects
        };
        SessionResume::Fallback(ResumeFallback {
            reason,
            block_id: self.block_id,
            block_available,
            landmark: self.landmark,
            ended: self.ended,
            deferred_effects,
            pending_effect: self.pending_effect,
            selected_choice_history: self.selected_choice_history,
            locale: self.locale,
        })
    }
}

fn change_level(saved: &DialogueBlockDigestsSnapshot, current: BlockDigests) -> ResumeLevel {
    let current = current.to_snapshot();
    if saved.content == current.content {
        ResumeLevel::UnchangedBlock
    } else if saved.structure == current.structure {
        ResumeLevel::TextEdited
    } else {
        ResumeLevel::Relocated
    }
}

fn find_block(
    view: AssetView<'_>,
    block_id: &BlockId,
) -> Result<Option<BlockIndex>, DialogueError> {
    match view.lookup_block(block_id.as_str()) {
        Ok(index) => Ok(Some(index)),
        Err(DialogueError::UnknownBlock { .. }) => Ok(None),
        Err(error) => Err(error),
    }
}

pub(crate) fn landmark_from_snapshot(
    saved: &DialogueLandmarkSnapshot,
) -> Result<ResumeLandmark, DialogueError> {
    let line_id = |value: &str| {
        recite_core::LineId::new(value).map_err(|error| invalid_snapshot(error.to_string()))
    };
    Ok(match saved {
        DialogueLandmarkSnapshot::Line { id } => ResumeLandmark::Line(line_id(id)?),
        DialogueLandmarkSnapshot::Prompt { line, choices } => ResumeLandmark::Prompt {
            line: line.as_deref().map(line_id).transpose()?,
            choices: choice_ids(choices)?,
        },
    })
}

fn choice_ids(values: &[String]) -> Result<Vec<ChoiceId>, DialogueError> {
    values.iter().map(|value| choice_id(value)).collect()
}
