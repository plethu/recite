use recite_core::{StatementIndex, StatementRange};

use crate::DialogueError;

use super::references::invalid_snapshot;

pub(super) fn validate_pending_statement_position(
    kind: &'static str,
    statement: StatementIndex,
    current_range: StatementRange,
    next_statement: StatementIndex,
) -> Result<(), DialogueError> {
    let range_start = current_range.start.as_u32();
    let range_end = range_start
        .checked_add(current_range.len)
        .ok_or_else(|| invalid_snapshot("active range overflows u32"))?;
    let statement = statement.as_u32();
    let expected_next = statement
        .checked_add(1)
        .ok_or_else(|| invalid_snapshot(format!("pending {kind} statement overflows u32")))?;

    if statement < range_start || statement >= range_end {
        return Err(invalid_snapshot(format!(
            "pending {kind} statement is outside the active range",
        )));
    }
    if next_statement.as_u32() != expected_next {
        return Err(invalid_snapshot(format!(
            "pending {kind} must be immediately before the restored next statement",
        )));
    }

    Ok(())
}
