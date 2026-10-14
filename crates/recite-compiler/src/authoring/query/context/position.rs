use recite_core::{DocumentKey, SourceLineIndex, SourcePosition, SourceSpan};

#[derive(Clone, Copy)]
pub(super) struct Line<'a> {
    pub text: &'a str,
    number: u32,
}

pub(super) fn line_at(index: &SourceLineIndex, number: u32) -> Option<Line<'_>> {
    let line = index.line(usize::try_from(number.checked_sub(1)?).ok()?)?;
    Some(Line {
        text: line.strip_suffix('\r').unwrap_or(line),
        number,
    })
}

pub(super) fn byte_at_column(line: &str, column: u32) -> usize {
    line.char_indices()
        .nth(column.saturating_sub(1) as usize)
        .map_or(line.len(), |(index, _)| index)
}

pub(super) fn span(key: &DocumentKey, source: Line<'_>, start: usize, end: usize) -> SourceSpan {
    let text = source.text;
    let start = start.min(text.len());
    let end = end.min(text.len()).max(start);
    let end = if end == start {
        start
    } else {
        text[..end]
            .char_indices()
            .next_back()
            .map_or(start, |(index, _)| index)
    };
    SourceSpan::new(
        key.as_str(),
        position(source, start),
        Some(position(source, end)),
    )
}

pub(super) fn token_span(
    key: &DocumentKey,
    source: Line<'_>,
    start: usize,
    cursor: usize,
) -> SourceSpan {
    let line = source.text;
    let cursor = cursor.max(start).min(line.len());
    let mut token_start = cursor;
    while token_start > start {
        let (index, character) = line[..token_start]
            .char_indices()
            .next_back()
            .unwrap_or((start, ' '));
        if !(character.is_alphanumeric() || matches!(character, '_' | '.' | '-')) {
            break;
        }
        token_start = index;
    }
    span(key, source, token_start, cursor)
}

pub(super) fn assignment_span(
    key: &DocumentKey,
    source: Line<'_>,
    assignment: recite_parser::MetadataAssignment<'_>,
) -> SourceSpan {
    let key_start = assignment
        .value_start
        .saturating_sub(assignment.key.len() + 1);
    span(
        key,
        source,
        key_start,
        assignment.end.min(source.text.len()),
    )
}

fn position(source: Line<'_>, offset: usize) -> SourcePosition {
    let column = source.text[..offset].chars().count() + 1;
    let Some(position) = u32::try_from(column)
        .ok()
        .and_then(|column| SourcePosition::new(source.number, column).ok())
    else {
        unreachable!("computed source position exceeded the typed range")
    };
    position
}

#[cfg(test)]
mod tests;
