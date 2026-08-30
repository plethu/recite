use lsp_types::{Position, Range};
use recite_compiler::SourceRange;
use recite_core::{SourcePosition, SourceSpan};

pub(crate) fn span_to_range(text: &str, span: &SourceSpan) -> Range {
    let start = source_position_to_lsp(text, span.start);
    let end = span
        .end
        .map(|position| source_position_to_lsp(text, advance_inclusive_end(text, position)))
        .unwrap_or(start);

    Range { start, end }
}

pub(crate) fn source_position_to_lsp(text: &str, position: SourcePosition) -> Position {
    let lines = DocumentLines::new(text);
    let line_index = position
        .line()
        .saturating_sub(1)
        .min(lines.last_line_index());
    let line = lines.line(line_index);
    let character = utf16_offset_for_scalar_column(line, position.column());

    Position {
        line: line_index,
        character,
    }
}

pub(crate) fn source_range_to_lsp(text: &str, range: SourceRange) -> Option<Range> {
    let start = exact_source_position_to_lsp(text, range.start())?;
    let end = exact_source_position_to_lsp(text, range.end())?;
    (start <= end).then_some(Range { start, end })
}

fn exact_source_position_to_lsp(text: &str, position: SourcePosition) -> Option<Position> {
    let line_index = usize::try_from(position.line().checked_sub(1)?).ok()?;
    let raw_line = text.split('\n').nth(line_index)?;
    let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
    let scalar_column = usize::try_from(position.column().checked_sub(1)?).ok()?;
    if scalar_column > line.chars().count() {
        return None;
    }
    let character = line
        .chars()
        .take(scalar_column)
        .map(char::len_utf16)
        .try_fold(0_u32, |total, width| {
            total.checked_add(u32::try_from(width).ok()?)
        })?;
    Some(Position::new(position.line().checked_sub(1)?, character))
}

/// Converts an LSP UTF-16 position to the compiler's 1-based scalar position.
///
/// LSP positions may address the middle of a UTF-16 surrogate pair.  The
/// compiler has no such position, so those cursors snap to the containing
/// scalar's start.  CRLF's carriage return is excluded from the logical line,
/// matching the source spans produced by the parser.
pub(crate) fn lsp_position_to_source(text: &str, position: Position) -> Option<SourcePosition> {
    let line_index = usize::try_from(position.line).ok()?;
    let raw_line = text.split('\n').nth(line_index)?;
    let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
    let column = scalar_column_for_utf16(line, position.character)?;
    SourcePosition::new(position.line.saturating_add(1), column).ok()
}

/// Returns source cursor positions represented by an LSP action range.
///
/// The compiler edit planner remains the authority for whether a position is
/// meaningful.  This helper only walks the exact scalar cursor locations in
/// the protocol range, which accommodates diagnostic ranges that begin on a
/// delimiter immediately before the semantic token.
pub(crate) fn source_positions_in_lsp_range(
    text: &str,
    range: Range,
) -> Option<Vec<SourcePosition>> {
    let start = lsp_position_to_source(text, range.start)?;
    let end = lsp_position_to_source(text, range.end)?;
    if start > end {
        return None;
    }
    if start == end {
        return Some(vec![start]);
    }

    let mut positions = Vec::new();
    let mut current = start;
    while current < end {
        positions.push(current);
        current = next_source_position(text, current)?;
    }
    Some(positions)
}

fn next_source_position(text: &str, position: SourcePosition) -> Option<SourcePosition> {
    let line_index = usize::try_from(position.line().checked_sub(1)?).ok()?;
    let raw_line = text.split('\n').nth(line_index)?;
    let line = raw_line.strip_suffix('\r').unwrap_or(raw_line);
    let scalar_count = u32::try_from(line.chars().count()).ok()?;
    if position.column() <= scalar_count {
        return SourcePosition::new(position.line(), position.column().checked_add(1)?).ok();
    }
    if position.column() == scalar_count.checked_add(1)? {
        return text
            .split('\n')
            .nth(line_index.checked_add(1)?)
            .and_then(|_| SourcePosition::new(position.line().checked_add(1)?, 1).ok());
    }
    None
}

fn scalar_column_for_utf16(line: &str, character: u32) -> Option<u32> {
    let mut utf16 = 0_u32;
    let mut scalar = 1_u32;
    for value in line.chars() {
        if utf16 == character {
            return Some(scalar);
        }
        let next = utf16.saturating_add(u32::try_from(value.len_utf16()).ok()?);
        if character < next {
            return Some(scalar);
        }
        utf16 = next;
        scalar = scalar.saturating_add(1);
    }
    (utf16 == character).then_some(scalar)
}

fn advance_inclusive_end(text: &str, position: SourcePosition) -> SourcePosition {
    let lines = DocumentLines::new(text);
    let line_index = position
        .line()
        .saturating_sub(1)
        .min(lines.last_line_index());
    let line = lines.line(line_index);
    let scalar_count = u32::try_from(line.chars().count()).unwrap_or(u32::MAX);
    let next_column = position
        .column()
        .saturating_add(1)
        .min(scalar_count.saturating_add(1));

    SourcePosition::new(line_index.saturating_add(1), next_column).unwrap_or(position)
}

fn utf16_offset_for_scalar_column(line: &str, column: u32) -> u32 {
    let scalar_prefix_len = usize::try_from(column.saturating_sub(1)).unwrap_or(usize::MAX);
    line.chars()
        .take(scalar_prefix_len)
        .map(char::len_utf16)
        .fold(0u32, |total, width| {
            total.saturating_add(u32::try_from(width).unwrap_or(u32::MAX))
        })
}

struct DocumentLines<'a> {
    lines: Vec<&'a str>,
}

impl<'a> DocumentLines<'a> {
    fn new(text: &'a str) -> Self {
        let lines = if text.is_empty() {
            vec![""]
        } else {
            text.split('\n').collect::<Vec<_>>()
        };

        Self { lines }
    }

    fn last_line_index(&self) -> u32 {
        u32::try_from(self.lines.len().saturating_sub(1)).unwrap_or(u32::MAX)
    }

    fn line(&self, index: u32) -> &'a str {
        usize::try_from(index)
            .ok()
            .and_then(|index| self.lines.get(index).copied())
            .unwrap_or("")
    }
}
