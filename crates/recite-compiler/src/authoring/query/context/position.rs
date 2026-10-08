use recite_core::{DocumentKey, SourcePosition, SourceSpan, source_lines};

pub(super) fn line_at(text: &str, line_number: u32) -> Option<(&str, usize)> {
    let mut offset = 0;
    for (index, (content, terminator)) in source_lines(text).enumerate() {
        if u32::try_from(index + 1).ok()? == line_number {
            return Some((content, offset));
        }
        offset += content.len() + terminator.len();
    }
    None
}

pub(super) fn byte_at_column(line: &str, column: u32) -> usize {
    line.char_indices()
        .nth(column.saturating_sub(1) as usize)
        .map_or(line.len(), |(index, _)| index)
}

pub(super) fn span(key: &DocumentKey, text: &str, start: usize, end: usize) -> SourceSpan {
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
        position(text, start),
        Some(position(text, end)),
    )
}

pub(super) fn token_span(
    key: &DocumentKey,
    text: &str,
    line: &str,
    line_start: usize,
    start: usize,
    cursor: usize,
) -> SourceSpan {
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
    span(key, text, line_start + token_start, line_start + cursor)
}

pub(super) fn assignment_span(
    key: &DocumentKey,
    text: &str,
    line: &str,
    line_start: usize,
    assignment: recite_parser::MetadataAssignment<'_>,
) -> SourceSpan {
    let key_start = assignment
        .value_start
        .saturating_sub(assignment.key.len() + 1);
    span(
        key,
        text,
        line_start + key_start,
        line_start + assignment.end.min(line.len()),
    )
}

fn position(text: &str, offset: usize) -> SourcePosition {
    let mut line = 1u32;
    let mut column = 1u32;
    for (index, character) in text[..offset.min(text.len())].char_indices() {
        if character == '\n'
            || (character == '\r' && text.as_bytes().get(index + 1) != Some(&b'\n'))
        {
            line += 1;
            column = 1;
        } else {
            column += 1;
        }
    }
    let Ok(position) = SourcePosition::new(line, column) else {
        unreachable!("computed source position exceeded the typed range")
    };
    position
}
