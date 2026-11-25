use recite_core::compiled::{
    CompiledEffectMode, CompiledStatementKind, StatementIndex, StatementRange,
};

use crate::DialogueError;
use crate::session::PendingEffect;
use crate::session_snapshot::DialogueSessionPendingEffectSnapshot;
use crate::traversal::{
    AssetView, dialogue_effect_request, runtime_effect_request_for_trace_counter,
};

use super::pending_position::validate_pending_statement_position;
use super::references::{invalid_snapshot, invalid_snapshot_with_source, snapshot_reference};
use crate::session_snapshot::effect_request_from_snapshot;

pub(super) fn restore_pending_effect(
    asset: AssetView<'_>,
    snapshot: Option<&DialogueSessionPendingEffectSnapshot>,
    has_pending_prompt: bool,
    ended: bool,
    current_range: StatementRange,
    next_statement: StatementIndex,
    trace_counter: u64,
) -> Result<Option<PendingEffect>, DialogueError> {
    let Some(snapshot) = snapshot else {
        return Ok(None);
    };

    if ended {
        return Err(invalid_snapshot(
            "ended sessions cannot have a pending effect",
        ));
    }
    if has_pending_prompt {
        return Err(invalid_snapshot(
            "sessions cannot have both a pending prompt and a pending effect",
        ));
    }
    if trace_counter == 0 {
        return Err(invalid_snapshot(
            "pending effect requires a nonzero trace counter",
        ));
    }

    let statement_index = StatementIndex::new(snapshot.statement);
    validate_pending_statement_position("effect", statement_index, current_range, next_statement)?;
    let statement = snapshot_reference(
        "pending effect statement",
        asset.statement_at(statement_index),
    )?;
    let CompiledStatementKind::Effect(effect_index) = &statement.kind else {
        return Err(invalid_snapshot(format!(
            "pending effect statement {} is not an effect",
            statement_index.as_u32()
        )));
    };
    let effect = snapshot_reference("pending effect", asset.effect_at(*effect_index))?;
    if effect.mode != CompiledEffectMode::Blocking {
        return Err(invalid_snapshot(format!(
            "pending effect `{}` is not a blocking effect",
            effect.id
        )));
    }

    let saved =
        effect_request_from_snapshot(&snapshot.request).map_err(invalid_snapshot_with_source)?;
    let request = runtime_effect_request_for_trace_counter(
        dialogue_effect_request(asset, effect)?,
        trace_counter,
    )?;
    if request.id != saved.id {
        return Err(invalid_snapshot(format!(
            "pending effect id `{}` does not match restored effect `{}`",
            saved.id, request.id
        )));
    }
    if request != saved {
        return Err(invalid_snapshot(format!(
            "pending effect request `{}` differs from its compiled effect",
            saved.id
        )));
    }

    Ok(Some(PendingEffect {
        statement: statement_index,
        request,
        reemit_on_next: true,
    }))
}
