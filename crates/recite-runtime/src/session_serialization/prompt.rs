use recite_core::{ChoiceId, CompiledStatementKind, StatementIndex, StatementRange};

use crate::DialogueError;
use crate::session::{PendingPrompt, PendingPromptChoice};
use crate::session_snapshot::{DialogueSessionPendingPromptSnapshot, availability_from_snapshot};
use crate::traversal::AssetView;

use super::pending_position::validate_pending_statement_position;
use super::references::{choice_id, invalid_snapshot, snapshot_reference};

pub(super) fn restore_pending_prompt(
    asset: AssetView<'_>,
    snapshot: Option<&DialogueSessionPendingPromptSnapshot>,
    previous_prompt_choices: &[ChoiceId],
    ended: bool,
    current_range: StatementRange,
    next_statement: StatementIndex,
) -> Result<Option<PendingPrompt>, DialogueError> {
    let Some(snapshot) = snapshot else {
        return Ok(None);
    };
    if ended {
        return Err(invalid_snapshot(
            "ended sessions cannot have a pending prompt",
        ));
    }
    if snapshot.choices.is_empty() {
        return Err(invalid_snapshot("pending prompt has no choices"));
    }

    let statement_index = StatementIndex::new(snapshot.statement);
    validate_pending_statement_position("prompt", statement_index, current_range, next_statement)?;
    let statement = snapshot_reference(
        "pending prompt statement",
        asset.statement_at(statement_index),
    )?;
    let CompiledStatementKind::Prompt { choices, .. } = &statement.kind else {
        return Err(invalid_snapshot(format!(
            "pending prompt statement {} is not a prompt",
            statement_index.as_u32()
        )));
    };
    let compiled_choices = asset.choices(*choices)?;
    if compiled_choices.len() != snapshot.choices.len() {
        return Err(invalid_snapshot(format!(
            "pending prompt choice count {} does not match compiled prompt choice count {}",
            snapshot.choices.len(),
            compiled_choices.len()
        )));
    }

    let mut pending_choices = Vec::with_capacity(snapshot.choices.len());
    for (compiled_choice, snapshot_choice) in compiled_choices.iter().zip(&snapshot.choices) {
        let choice_id = choice_id(&snapshot_choice.id)?;
        if compiled_choice.id != choice_id {
            return Err(invalid_snapshot(format!(
                "pending prompt choice `{}` does not match compiled choice `{}`",
                snapshot_choice.id, compiled_choice.id
            )));
        }
        let availability = availability_from_snapshot(snapshot_choice.availability.clone())
            .map_err(|error| invalid_snapshot(error.to_string()))?;
        pending_choices.push(PendingPromptChoice {
            id: choice_id,
            target: compiled_choice.target.clone(),
            is_available: availability.is_available,
            availability,
        });
    }

    let pending_choice_ids = pending_choices
        .iter()
        .map(|choice| choice.id.clone())
        .collect::<Vec<_>>();
    if pending_choice_ids != previous_prompt_choices {
        return Err(invalid_snapshot(
            "pending prompt choices must match previous prompt choices",
        ));
    }

    Ok(Some(PendingPrompt {
        statement: statement_index,
        choices: pending_choices,
    }))
}
