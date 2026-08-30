use recite_core::{ChoiceId, LocaleId};

use super::PreviewSession;
use super::model::{
    PREVIEW_SNAPSHOT_FORMAT_VERSION, PreviewError, PreviewEvent, PreviewSnapshot, PreviewState,
    PreviewStatus,
};
use super::snapshot_fingerprint::projection_fingerprint;
use crate::{restore_session, snapshot_session};

impl<'asset> PreviewSession<'asset> {
    /// Captures stable runtime state. Trace and transcript are diagnostic
    /// projections and are deliberately excluded from the snapshot.
    pub fn snapshot(&self) -> Result<PreviewSnapshot, PreviewError> {
        if self.pending.is_some() {
            return Err(PreviewError::SnapshotPendingCondition);
        }
        Ok(PreviewSnapshot {
            snapshot_format_version: PREVIEW_SNAPSHOT_FORMAT_VERSION,
            session: snapshot_session(&self.session),
            initial_block: self.block.clone(),
            options: self.options.clone(),
            next_condition_id: self.next_condition_id,
            projection_fingerprint: projection_fingerprint(&self.state),
            state: self.state.clone(),
        })
    }

    /// Restores stable runtime state without importing trace or transcript.
    pub fn restore(
        &mut self,
        snapshot: PreviewSnapshot,
    ) -> Result<super::model::PreviewOutput, PreviewError> {
        if snapshot.snapshot_format_version() != PREVIEW_SNAPSHOT_FORMAT_VERSION {
            return Err(PreviewError::UnsupportedSnapshotFormat {
                snapshot_format_version: snapshot.snapshot_format_version(),
            });
        }
        if snapshot.state().asset_id() != &self.asset.header.asset_id {
            return Err(PreviewError::SnapshotAssetMismatch {
                expected: self.asset.header.asset_id.clone(),
                actual: snapshot.state().asset_id().clone(),
            });
        }
        if snapshot.initial_block().is_some_and(|block| {
            !self
                .asset
                .blocks
                .iter()
                .any(|candidate| candidate.id.as_str() == block)
        }) {
            return Err(PreviewError::SnapshotStateMismatch);
        }
        if snapshot.state().status().is_waiting_for_condition() {
            return Err(PreviewError::SnapshotPendingCondition);
        }
        if snapshot.projection_fingerprint() != projection_fingerprint(snapshot.state()) {
            return Err(PreviewError::SnapshotStateMismatch);
        }
        let restored = restore_session(self.asset, snapshot.session().clone())
            .map_err(PreviewError::Runtime)?;
        if snapshot.options().locale().map(LocaleId::as_str)
            != restored.locale().map(LocaleId::as_str)
            || snapshot.state().locale().map(LocaleId::as_str)
                != restored.locale().map(LocaleId::as_str)
        {
            return Err(PreviewError::SnapshotStateMismatch);
        }
        if !snapshot_state_matches_session(self.asset, snapshot.state(), &restored) {
            return Err(PreviewError::SnapshotStateMismatch);
        }
        self.session = restored;
        self.block = snapshot.initial_block.clone();
        self.options = snapshot.options.clone();
        self.next_condition_id = snapshot.next_condition_id;
        self.state = snapshot.state().clone();
        self.pending = None;
        self.restored_effect_reemit = match self.state.status() {
            PreviewStatus::WaitingForEffect { effect }
                if effect.mode == crate::DialogueEffectMode::Blocking =>
            {
                Some(effect.id.clone())
            }
            _ => None,
        };
        self.trace = super::model::PreviewTrace::new(
            self.session.locale().cloned(),
            self.options.variant.clone(),
        );
        self.transcript = super::model::PreviewTranscript::default();
        Ok(self.append_events(vec![PreviewEvent::Restored]))
    }
}

fn snapshot_state_matches_session(
    asset: &recite_core::CompiledDialogue,
    state: &PreviewState,
    session: &crate::DialogueSession,
) -> bool {
    let snapshot = snapshot_session(session);
    let active_block = asset
        .blocks
        .get(session.active_block_index().as_u32() as usize)
        .map(|block| block.id.clone());
    if state
        .restart_required()
        .is_some_and(|requirement| requirement.active_asset() != state.asset_id())
    {
        return false;
    }
    if state.block() != active_block.as_ref()
        || state.locale().map(LocaleId::as_str) != snapshot.locale.as_deref()
        || state
            .selected_choice_history()
            .iter()
            .map(ChoiceId::as_str)
            .collect::<Vec<_>>()
            != snapshot
                .selected_choice_history
                .iter()
                .map(String::as_str)
                .collect::<Vec<_>>()
        || state.deferred_effects() != session.deferred_effects()
    {
        return false;
    }
    match state.status() {
        PreviewStatus::WaitingForChoice { prompt } => {
            state.block() == Some(prompt.identity().block())
                && snapshot.pending_prompt.as_ref().is_some_and(|saved| {
                    let statement = asset.statements.get(saved.statement as usize);
                    let Some(recite_core::CompiledStatementKind::Prompt { line, choices }) =
                        statement.map(|statement| &statement.kind)
                    else {
                        return false;
                    };
                    let compiled_line =
                        line.and_then(|index| asset.lines.get(index.as_u32() as usize));
                    let prompt_line = compiled_line.map(|line| line.id.clone());
                    let start = choices.start.as_u32() as usize;
                    let Some(end) = start.checked_add(choices.len as usize) else {
                        return false;
                    };
                    let Some(compiled_choices) = asset.choices.get(start..end) else {
                        return false;
                    };
                    prompt.identity().line() == prompt_line.as_ref()
                        && match (prompt_line.as_ref(), prompt.line()) {
                            (None, None) => true,
                            (Some(expected), Some(line)) => {
                                compiled_line.is_some_and(|candidate| {
                                    &candidate.id == expected
                                        && (candidate.source_text == line.source_text
                                            || candidate.plural_source_text.as_deref()
                                                == Some(line.source_text.as_str()))
                                        && candidate.id == line.id
                                })
                            }
                            _ => false,
                        }
                        && prompt.identity().choices()
                            == compiled_choices
                                .iter()
                                .map(|choice| choice.id.clone())
                                .collect::<Vec<_>>()
                        && saved.choices.len() == prompt.choices().len()
                        && saved
                            .choices
                            .iter()
                            .map(|choice| choice.id.as_str())
                            .eq(prompt.choices().iter().map(|choice| choice.id.as_str()))
                        && saved
                            .choices
                            .iter()
                            .map(|choice| choice.id.as_str())
                            .eq(prompt.identity().choices().iter().map(ChoiceId::as_str))
                        && saved
                            .choices
                            .iter()
                            .zip(prompt.choices())
                            .zip(compiled_choices)
                            .all(|((saved, choice), compiled)| {
                                crate::session_snapshot::availability_snapshot(&choice.availability)
                                    == saved.availability
                                    && compiled.id == choice.id
                                    && compiled.source_text == choice.source_text
                            })
                })
        }
        PreviewStatus::WaitingForEffect { effect } => {
            snapshot.pending_effect.as_ref().is_some_and(|saved| {
                saved.id == effect.id.as_str() && session.pending_effect() == Some(effect)
            })
        }
        PreviewStatus::Ended => snapshot.ended,
        PreviewStatus::WaitingForCondition { .. } => false,
        PreviewStatus::Ready => {
            snapshot.pending_prompt.is_none()
                && snapshot.pending_effect.is_none()
                && !snapshot.ended
        }
    }
}
