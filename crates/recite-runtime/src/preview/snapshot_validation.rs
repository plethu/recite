use recite_core::{ChoiceId, CompiledChoiceEcho, LocaleId};

use super::model::{PreviewState, PreviewStatus};
use crate::{DialogueChoice, DialogueLine, DialogueSession, DialogueSessionSnapshot};

pub(super) fn state_matches_session(
    asset: &recite_core::CompiledDialogue,
    state: &PreviewState,
    session: &DialogueSession,
) -> bool {
    let snapshot = crate::snapshot_session(session);
    let active_block = asset
        .blocks
        .get(session.active_block_index().as_u32() as usize)
        .map(|block| block.id.clone());
    if state.restart_required().is_some_and(|requirement| {
        requirement.active_asset() != state.asset_id()
            || super::model::PreviewAssetRevision::from_asset(asset)
                .map_or(true, |revision| requirement.active_revision() != &revision)
    }) {
        return false;
    }
    if state.block() != active_block.as_ref()
        || state.locale().map(LocaleId::as_str) != snapshot.locale.as_deref()
        || !selected_history_matches(state, &snapshot)
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
                    let active_block = asset
                        .blocks
                        .get(session.active_block_index().as_u32() as usize);
                    let compiled_line =
                        line.and_then(|index| asset.lines.get(index.as_u32() as usize));
                    let start = choices.start.as_u32() as usize;
                    let Some(end) = start.checked_add(choices.len as usize) else {
                        return false;
                    };
                    let Some(compiled_choices) = asset.choices.get(start..end) else {
                        return false;
                    };
                    prompt_matches_line(asset, active_block, prompt.line(), compiled_line)
                        && prompt.identity().line() == compiled_line.map(|compiled| &compiled.id)
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
                            .zip(prompt.choices())
                            .zip(compiled_choices)
                            .all(|((saved, choice), compiled)| {
                                crate::session_snapshot::availability_snapshot(&choice.availability)
                                    == saved.availability
                                    && choice_projection_matches(asset, choice, compiled)
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

fn selected_history_matches(state: &PreviewState, snapshot: &DialogueSessionSnapshot) -> bool {
    state
        .selected_choice_history()
        .iter()
        .map(ChoiceId::as_str)
        .eq(snapshot.selected_choice_history.iter().map(String::as_str))
}

fn prompt_matches_line(
    asset: &recite_core::CompiledDialogue,
    active_block: Option<&recite_core::CompiledBlock>,
    projected: Option<&DialogueLine>,
    compiled: Option<&recite_core::CompiledLine>,
) -> bool {
    match (projected, compiled) {
        (None, None) => true,
        (Some(line), Some(compiled)) => {
            let expected_source = if line.source_text == compiled.source_text {
                Some(0)
            } else if compiled.plural_source_text.as_deref() == Some(line.source_text.as_str()) {
                Some(1)
            } else {
                None
            };
            let expected_speaker = compiled
                .speaker
                .or_else(|| active_block.and_then(|block| block.default_speaker))
                .and_then(|index| asset.speakers.get(index.as_u32() as usize))
                .map(|speaker| &speaker.id);
            expected_source.is_some()
                && line.id == compiled.id
                && line.speaker.as_ref() == expected_speaker
                && line.key_metadata_matches(asset, compiled.metadata)
                && match (
                    compiled.authored_plural_source_text.as_ref(),
                    line.plural.as_ref(),
                ) {
                    (Some(authored_plural), Some(plural)) => {
                        plural.selected_arm <= 1
                            && expected_source == Some(plural.selected_arm)
                            && plural.singular_source_text == compiled.authored_source_text
                            && plural.plural_source_text == *authored_plural
                    }
                    (None, None) => true,
                    _ => false,
                }
        }
        _ => false,
    }
}

fn choice_projection_matches(
    asset: &recite_core::CompiledDialogue,
    projected: &DialogueChoice,
    compiled: &recite_core::CompiledChoice,
) -> bool {
    projected.id == compiled.id
        && projected.source_text == compiled.source_text
        && projected.key_metadata_matches(asset, compiled.metadata)
        && match (&projected.echo, &compiled.echo) {
            (crate::ChoiceEchoMode::None, CompiledChoiceEcho::None)
            | (crate::ChoiceEchoMode::SelectedText, CompiledChoiceEcho::SelectedText) => true,
            (
                crate::ChoiceEchoMode::ExplicitLine(actual),
                CompiledChoiceEcho::ExplicitLine(expected),
            ) => actual == expected,
            _ => false,
        }
}

trait MetadataProjection {
    fn key_metadata_matches(
        &self,
        asset: &recite_core::CompiledDialogue,
        range: recite_core::MetadataRange,
    ) -> bool;
}

impl MetadataProjection for DialogueLine {
    fn key_metadata_matches(
        &self,
        asset: &recite_core::CompiledDialogue,
        range: recite_core::MetadataRange,
    ) -> bool {
        metadata_matches(asset, range, &self.metadata)
    }
}

impl MetadataProjection for DialogueChoice {
    fn key_metadata_matches(
        &self,
        asset: &recite_core::CompiledDialogue,
        range: recite_core::MetadataRange,
    ) -> bool {
        metadata_matches(asset, range, &self.metadata)
    }
}

fn metadata_matches(
    asset: &recite_core::CompiledDialogue,
    range: recite_core::MetadataRange,
    projected: &[recite_core::MetadataEntry],
) -> bool {
    let start = range.start.as_u32() as usize;
    let Some(end) = start.checked_add(range.len as usize) else {
        return false;
    };
    let Some(expected) = asset.metadata.get(start..end) else {
        return false;
    };
    expected.len() == projected.len()
        && expected.iter().zip(projected).all(|(expected, actual)| {
            let span = expected
                .source_map
                .and_then(|index| asset.source_maps.get(index.as_u32() as usize))
                .map(|entry| &entry.span);
            expected.key == actual.key
                && expected.value == actual.value
                && actual.source_span.as_ref() == span
                && actual.key_span.is_none()
                && actual.value_span.is_none()
        })
}
