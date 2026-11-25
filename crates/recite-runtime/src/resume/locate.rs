//! Find a saved position again in a patched asset.

use recite_core::compiled::{
    CompiledBlock, CompiledMatchPattern, CompiledStatementKind, StatementIndex, StatementRange,
};

use crate::DialogueError;
use crate::session::{
    PendingEffect, PendingPrompt, PendingPromptChoice, SessionPhase, StatementFrame,
};
use crate::session_serialization::references::{invalid_snapshot, invalid_snapshot_with_source};
use crate::session_snapshot::{
    DialogueBranchSnapshot, DialogueEffectModeSnapshot, DialogueLandmarkSnapshot,
    DialoguePositionAnchorSnapshot, DialogueSessionSnapshot, DialogueStatementSignatureSnapshot,
    availability_from_snapshot, effect_request_from_snapshot,
};
use crate::traversal::{AssetView, dialogue_effect_request};

use super::key::{Landmark, StatementKey, statement_key};
use super::outcome::{PromptResume, ResumeFallbackReason};
use super::relocate::landmark_from_snapshot;

pub(super) struct LocatedPosition {
    pub(super) frames: Vec<StatementFrame>,
    pub(super) current_range: StatementRange,
    pub(super) next_statement: StatementIndex,
    pub(super) phase: SessionPhase,
    pub(super) prompt: PromptResume,
}

/// Relocates the saved position. The outer error is a hard failure; the
/// inner one is a structured fallback reason.
pub(super) fn locate_session(
    view: AssetView<'_>,
    block: &CompiledBlock,
    snapshot: &DialogueSessionSnapshot,
    structure_unchanged: bool,
) -> Result<Result<LocatedPosition, ResumeFallbackReason>, DialogueError> {
    if snapshot.ended {
        let range = block.statements;
        return Ok(Ok(LocatedPosition {
            frames: Vec::new(),
            current_range: range,
            next_statement: range_end(range)?,
            phase: SessionPhase::Ended,
            prompt: PromptResume::None,
        }));
    }

    let anchor = &snapshot.resume_anchor;
    let mut range = block.statements;
    let mut frames = Vec::with_capacity(anchor.frames.len());
    for frame in &anchor.frames {
        if frame.position.trail.is_empty() {
            return Err(invalid_snapshot(
                "a resume frame must end at the branching statement it entered",
            ));
        }
        let next = match locate(view, range, &frame.position)? {
            Ok(next) => next,
            Err(reason) => return Ok(Err(reason)),
        };
        let child = match select_branch(view, previous(next)?, &frame.branch)? {
            Some(child) => child,
            None => return Ok(Err(ResumeFallbackReason::BranchRemoved)),
        };
        frames.push(StatementFrame {
            range,
            next_statement: next,
        });
        range = child;
    }
    let next = match locate(view, range, &anchor.position)? {
        Ok(next) => next,
        Err(reason) => return Ok(Err(reason)),
    };

    let (next_statement, phase, prompt) = if let Some(pending) = &snapshot.pending_prompt {
        let position = &anchor.position;
        if !matches!(
            position.landmark,
            Some(DialogueLandmarkSnapshot::Prompt { .. })
        ) || !position.trail.is_empty()
        {
            return Err(invalid_snapshot(
                "a pending prompt must be the landmark of the saved position",
            ));
        }
        if pending
            .choices
            .iter()
            .map(|choice| choice.id.as_str())
            .ne(snapshot.previous_prompt_choices.iter().map(String::as_str))
        {
            return Err(invalid_snapshot(
                "pending prompt choices must match previous prompt choices",
            ));
        }
        let statement = previous(next)?;
        let CompiledStatementKind::Prompt { choices, .. } = &view.statement_at(statement)?.kind
        else {
            return Err(invalid_snapshot("relocated pending prompt is not a prompt"));
        };
        let compiled = view.choices(*choices)?;
        let unchanged = structure_unchanged
            && compiled.len() == pending.choices.len()
            && compiled
                .iter()
                .zip(&pending.choices)
                .all(|(compiled, saved)| compiled.id.as_str() == saved.id);
        if unchanged {
            let mut choices = Vec::with_capacity(compiled.len());
            for (compiled, saved) in compiled.iter().zip(&pending.choices) {
                choices.push(PendingPromptChoice {
                    id: compiled.id.clone(),
                    target: compiled.target.clone(),
                    availability: availability_from_snapshot(saved.availability.clone())
                        .map_err(invalid_snapshot_with_source)?,
                });
            }
            (
                next,
                SessionPhase::AwaitingChoice(PendingPrompt {
                    statement,
                    choices,
                    reemit_on_next: true,
                }),
                PromptResume::Kept,
            )
        } else {
            (statement, SessionPhase::Running, PromptResume::Reevaluated)
        }
    } else if let Some(pending) = &snapshot.pending_effect {
        let blocking = matches!(
            anchor.position.trail.last(),
            Some(DialogueStatementSignatureSnapshot::Effect {
                mode: DialogueEffectModeSnapshot::Blocking,
                ..
            })
        );
        if !blocking || snapshot.trace_counter == 0 {
            return Err(invalid_snapshot(
                "a pending effect must be the blocking effect before the saved position",
            ));
        }
        let request =
            effect_request_from_snapshot(&pending.request).map_err(invalid_snapshot_with_source)?;
        // The trail matched the compiled effect's signature; the saved request
        // must still ask for that effect, as exact restore requires.
        let statement = previous(next)?;
        let CompiledStatementKind::Effect(effect) = &view.statement_at(statement)?.kind else {
            return Err(invalid_snapshot(
                "relocated pending effect is not an effect",
            ));
        };
        let compiled = dialogue_effect_request(view, view.effect_at(*effect)?)?;
        if request.mode != compiled.mode
            || request.function != compiled.function
            || request.args != compiled.args
        {
            return Err(invalid_snapshot(format!(
                "pending effect request `{}` differs from its compiled effect",
                request.id
            )));
        }
        (
            next,
            SessionPhase::AwaitingEffect(PendingEffect {
                statement,
                request,
                reemit_on_next: true,
            }),
            PromptResume::None,
        )
    } else {
        (next, SessionPhase::Running, PromptResume::None)
    };

    Ok(Ok(LocatedPosition {
        frames,
        current_range: range,
        next_statement,
        phase,
        prompt,
    }))
}

/// Finds the saved position in one statement range.
fn locate(
    view: AssetView<'_>,
    range: StatementRange,
    anchor: &DialoguePositionAnchorSnapshot,
) -> Result<Result<StatementIndex, ResumeFallbackReason>, DialogueError> {
    let start = range.start.as_u32();
    let end = range_end(range)?.as_u32();
    let base = match &anchor.landmark {
        None => start,
        Some(saved) => {
            let mut found = None;
            for index in start..end {
                if let StatementKey::Landmark(landmark) =
                    statement_key(view, StatementIndex::new(index))?
                    && landmark.matches(saved)
                {
                    if found.is_some() {
                        return Ok(Err(ResumeFallbackReason::LandmarkAmbiguous {
                            landmark: landmark_from_snapshot(saved)?,
                        }));
                    }
                    found = Some(index);
                }
            }
            match found {
                Some(index) => index + 1,
                None => {
                    let landmark = landmark_from_snapshot(saved)?;
                    return Ok(Err(if landmark_exists(view, saved) {
                        ResumeFallbackReason::LandmarkMoved { landmark }
                    } else {
                        ResumeFallbackReason::LandmarkRemoved { landmark }
                    }));
                }
            }
        }
    };
    // The landmark is the last line before the saved point, so any line
    // between it and the point was added by the patch. It precedes the point,
    // like a line added before the landmark, and is skipped.
    let mut index = base;
    for saved in &anchor.trail {
        loop {
            let key = if index < end {
                Some(statement_key(view, StatementIndex::new(index))?)
            } else {
                None
            };
            index += 1;
            match key {
                Some(StatementKey::Signature(signature)) if signature.matches(saved) => break,
                Some(StatementKey::Landmark(Landmark::Line { .. })) => {}
                _ => {
                    return Ok(Err(ResumeFallbackReason::StatementsChanged {
                        landmark: anchor
                            .landmark
                            .as_ref()
                            .map(landmark_from_snapshot)
                            .transpose()?,
                    }));
                }
            }
        }
    }
    Ok(Ok(StatementIndex::new(index)))
}

fn select_branch(
    view: AssetView<'_>,
    statement: StatementIndex,
    branch: &DialogueBranchSnapshot,
) -> Result<Option<StatementRange>, DialogueError> {
    Ok(match (&view.statement_at(statement)?.kind, branch) {
        (
            CompiledStatementKind::If {
                then_statements, ..
            },
            DialogueBranchSnapshot::Then,
        ) => Some(*then_statements),
        (
            CompiledStatementKind::If {
                else_statements, ..
            },
            DialogueBranchSnapshot::Else,
        ) => Some(*else_statements),
        (
            CompiledStatementKind::Match { arms, .. },
            DialogueBranchSnapshot::Arm {
                variant,
                occurrence,
            },
        ) => view
            .match_arms(*arms)?
            .iter()
            .filter(|arm| match (&arm.pattern, variant) {
                (CompiledMatchPattern::Variant(compiled), Some(saved)) => compiled == saved,
                (CompiledMatchPattern::Wildcard, None) => true,
                _ => false,
            })
            .nth(*occurrence as usize)
            .map(|arm| arm.statements),
        _ => None,
    })
}

fn landmark_exists(view: AssetView<'_>, saved: &DialogueLandmarkSnapshot) -> bool {
    match saved {
        DialogueLandmarkSnapshot::Line { id } => view.has_line(id),
        DialogueLandmarkSnapshot::Prompt { line, choices } => {
            line.as_deref().is_some_and(|line| view.has_line(line))
                || choices.iter().any(|choice| view.has_choice(choice))
        }
    }
}

fn previous(index: StatementIndex) -> Result<StatementIndex, DialogueError> {
    index
        .as_u32()
        .checked_sub(1)
        .map(StatementIndex::new)
        .ok_or_else(|| invalid_snapshot("saved position has no preceding statement"))
}

fn range_end(range: StatementRange) -> Result<StatementIndex, DialogueError> {
    range
        .start
        .as_u32()
        .checked_add(range.len)
        .map(StatementIndex::new)
        .ok_or_else(|| invalid_snapshot("statement range overflows u32"))
}
