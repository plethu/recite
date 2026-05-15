use recite_core::{CompiledDivertTarget, StatementIndex, StatementRange};

use crate::event::DialogueEvent;
use crate::session::StatementFrame;
use crate::{DialogueError, DialogueSession};

use super::{AssetView, malformed};

pub(super) fn finish_scene(session: &mut DialogueSession) -> Result<DialogueEvent, DialogueError> {
    session.ended = true;
    if let Some(root_frame) = session.continuation_stack.first() {
        session.current_range = root_frame.range;
    }
    session.next_statement = range_end_statement(session.current_range)?;
    session.continuation_stack.clear();
    let deferred_effects = session.deferred_effects.clone();
    session.emit(DialogueEvent::End { deferred_effects })
}

pub(super) fn apply_divert(
    asset: AssetView<'_>,
    session: &mut DialogueSession,
    target: &CompiledDivertTarget,
) -> Result<(), DialogueError> {
    match target {
        CompiledDivertTarget::Block(block_index) => {
            let block = asset.block_at(*block_index)?;
            session.current_block = *block_index;
            session.current_range = block.statements;
            session.next_statement = block.statements.start;
            session.continuation_stack.clear();
        }
        CompiledDivertTarget::End => unreachable!("end diverts are handled by caller"),
    }

    Ok(())
}

pub(super) fn enter_statement_range(
    asset: AssetView<'_>,
    session: &mut DialogueSession,
    range: StatementRange,
    continuation: StatementIndex,
) -> Result<(), DialogueError> {
    asset.statement_range(range)?;
    session.continuation_stack.push(StatementFrame {
        range: session.current_range,
        next_statement: continuation,
    });
    session.current_range = range;
    session.next_statement = range.start;

    Ok(())
}

pub(super) fn next_statement_after(index: StatementIndex) -> Result<StatementIndex, DialogueError> {
    index
        .as_u32()
        .checked_add(1)
        .map(StatementIndex::new)
        .ok_or_else(|| malformed("statement index overflowed".to_owned()))
}

fn range_end_statement(range: StatementRange) -> Result<StatementIndex, DialogueError> {
    range
        .start
        .as_u32()
        .checked_add(range.len)
        .map(StatementIndex::new)
        .ok_or_else(|| malformed("statement range end overflowed".to_owned()))
}
