use lsp_types::{Position, Range};
use recite_compiler::authoring::SourceRange;
use recite_core::{SourcePosition, SourceSpan, source_lines};

pub(crate) fn span_to_range(text: &str, span: &SourceSpan) -> Range {
    DocumentLines::new(text).span_to_range(span)
}

pub(crate) fn source_range_to_lsp(
    text: &recite_core::SourceLineIndex,
    range: SourceRange,
) -> Option<Range> {
    let start = exact_source_position_to_lsp(text, range.start())?;
    let end = exact_source_position_to_lsp(text, range.end())?;
    (start <= end).then_some(Range { start, end })
}

fn exact_source_position_to_lsp(
    text: &recite_core::SourceLineIndex,
    position: SourcePosition,
) -> Option<Position> {
    let line_index = usize::try_from(position.line().checked_sub(1)?).ok()?;
    let raw_line = text.line(line_index)?;
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
    let (line, _) = source_lines(text).nth(line_index)?;
    let column = scalar_column_for_utf16(line, position.character)?;
    SourcePosition::new(position.line.saturating_add(1), column).ok()
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

fn utf16_offset_for_scalar_column(line: &str, column: u32) -> u32 {
    let scalar_prefix_len = usize::try_from(column.saturating_sub(1)).unwrap_or(usize::MAX);
    line.chars()
        .take(scalar_prefix_len)
        .map(char::len_utf16)
        .fold(0u32, |total, width| {
            total.saturating_add(u32::try_from(width).unwrap_or(u32::MAX))
        })
}

pub(crate) struct DocumentLines<'a> {
    lines: Vec<&'a str>,
}

impl<'a> DocumentLines<'a> {
    pub(crate) fn new(text: &'a str) -> Self {
        let lines = source_lines(text).map(|(content, _)| content).collect();

        Self { lines }
    }

    pub(crate) fn span_to_range(&self, span: &SourceSpan) -> Range {
        let start = self.source_position_to_lsp(span.start);
        let end = span
            .end
            .map(|position| self.source_position_to_lsp(self.advance_inclusive_end(position)))
            .unwrap_or(start);
        Range { start, end }
    }

    fn source_position_to_lsp(&self, position: SourcePosition) -> Position {
        let line_index = position
            .line()
            .saturating_sub(1)
            .min(self.last_line_index());
        let character = utf16_offset_for_scalar_column(self.line(line_index), position.column());
        Position {
            line: line_index,
            character,
        }
    }

    fn advance_inclusive_end(&self, position: SourcePosition) -> SourcePosition {
        let line_index = position
            .line()
            .saturating_sub(1)
            .min(self.last_line_index());
        let line = self.line(line_index);
        let scalar_count = u32::try_from(line.chars().count()).unwrap_or(u32::MAX);
        let next_column = position
            .column()
            .saturating_add(1)
            .min(scalar_count.saturating_add(1));

        SourcePosition::new(line_index.saturating_add(1), next_column).unwrap_or(position)
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
