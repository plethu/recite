use lsp_types::{Position, Range};
use recite_core::{SourcePosition, SourceSpan};

pub(crate) fn span_to_range(text: &str, span: &SourceSpan) -> Range {
    let start = source_position_to_lsp(text, span.start);
    let end = span
        .end
        .map(|position| source_position_to_lsp(text, advance_inclusive_end(text, position)))
        .unwrap_or(start);

    Range { start, end }
}

fn source_position_to_lsp(text: &str, position: SourcePosition) -> Position {
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
