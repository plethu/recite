use recite_core::{CompiledStatementKind, StatementIndex, StatementRange};

use crate::DialogueError;
use crate::session::StatementFrame;
use crate::session_snapshot::{DialogueSessionFrameSnapshot, statement_range};
use crate::traversal::AssetView;

use super::references::{invalid_snapshot, snapshot_reference};

pub(super) fn restore_frames(
    asset: AssetView<'_>,
    snapshots: &[DialogueSessionFrameSnapshot],
) -> Result<Vec<StatementFrame>, DialogueError> {
    snapshots
        .iter()
        .map(|snapshot| {
            let range = statement_range(snapshot.range);
            snapshot_reference("continuation frame range", asset.statement_range(range))?;
            let next_statement = StatementIndex::new(snapshot.next_statement);
            validate_statement_pointer("continuation next statement", range, next_statement)?;

            Ok(StatementFrame {
                range,
                next_statement,
            })
        })
        .collect()
}

pub(super) fn validate_range_stack(
    asset: AssetView<'_>,
    block_range: StatementRange,
    current_range: StatementRange,
    frames: &[StatementFrame],
) -> Result<(), DialogueError> {
    if frames.is_empty() {
        if current_range != block_range {
            return Err(invalid_snapshot(
                "active statement range must match current block when continuation stack is empty",
            ));
        }
        return Ok(());
    }

    if frames[0].range != block_range {
        return Err(invalid_snapshot(
            "first continuation frame must resume within the current block range",
        ));
    }

    for (index, frame) in frames.iter().enumerate() {
        let child_range = frames
            .get(index + 1)
            .map_or(current_range, |next_frame| next_frame.range);
        validate_child_range(asset, frame.range, frame.next_statement, child_range)?;
    }

    Ok(())
}

fn validate_child_range(
    asset: AssetView<'_>,
    parent_range: StatementRange,
    continuation: StatementIndex,
    child_range: StatementRange,
) -> Result<(), DialogueError> {
    let parent_start = parent_range.start.as_u32();
    let parent_end = parent_start
        .checked_add(parent_range.len)
        .ok_or_else(|| invalid_snapshot("parent range overflows u32"))?;
    let Some(branch_statement) = continuation.as_u32().checked_sub(1) else {
        return Err(invalid_snapshot(
            "continuation frame cannot resume before statement zero",
        ));
    };

    if branch_statement < parent_start || branch_statement >= parent_end {
        return Err(invalid_snapshot(
            "continuation frame does not resume after a statement in its parent range",
        ));
    }

    let statement = asset.statement_at(StatementIndex::new(branch_statement))?;
    match &statement.kind {
        CompiledStatementKind::If {
            then_statements,
            else_statements,
            ..
        } => {
            if child_range != *then_statements && child_range != *else_statements {
                return Err(invalid_snapshot(
                    "active child range is not selected by its continuation frame",
                ));
            }

            Ok(())
        }
        CompiledStatementKind::Match { arms, .. } => {
            if asset
                .match_arms(*arms)?
                .iter()
                .any(|arm| arm.statements == child_range)
            {
                Ok(())
            } else {
                Err(invalid_snapshot(
                    "active child range is not selected by its continuation frame",
                ))
            }
        }
        _ => Err(invalid_snapshot(
            "continuation frame does not resume after a branching statement",
        )),
    }
}

pub(super) fn validate_statement_pointer(
    field: &'static str,
    range: StatementRange,
    pointer: StatementIndex,
) -> Result<(), DialogueError> {
    let start = range.start.as_u32();
    let end = start
        .checked_add(range.len)
        .ok_or_else(|| invalid_snapshot(format!("{field} range overflows u32")))?;
    let pointer = pointer.as_u32();

    if pointer < start || pointer > end {
        return Err(invalid_snapshot(format!(
            "{field} {pointer} is outside active range {start}..={end}"
        )));
    }

    Ok(())
}
