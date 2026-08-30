use recite_core::{DocumentKey, MetadataTarget, SourcePosition, SourceSpan};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum Site {
    Blocks {
        target: Option<DocumentKey>,
        token: SourceSpan,
    },
    Speakers(SourceSpan),
    MetadataKey {
        span: SourceSpan,
        target: MetadataTarget,
    },
    MetadataValue {
        key: String,
        token: SourceSpan,
        target: MetadataTarget,
    },
    Conditions(SourceSpan),
    Effects(SourceSpan),
    AvailabilityReasons(SourceSpan),
}

pub(super) fn at(key: &DocumentKey, text: &str, position: SourcePosition) -> Option<Site> {
    let (line, line_start) = line_at(text, position.line())?;
    let cursor = byte_at_column(line, position.column());
    let prefix = &line[..cursor];
    let trimmed = prefix.trim_start();
    let trim_offset = prefix.len() - trimmed.len();

    if trimmed.starts_with("->") {
        let mut start = trim_offset + 2;
        while line
            .get(start..)?
            .chars()
            .next()
            .is_some_and(char::is_whitespace)
        {
            start += line[start..].chars().next()?.len_utf8();
        }
        let token = span(key, text, line_start + start, line_start + cursor);
        let raw = line.get(start..cursor)?.trim();
        let target = raw
            .rsplit_once("::")
            .map(|(file, _)| DocumentKey::new(file.to_owned()).ok())
            .unwrap_or(None);
        return Some(Site::Blocks { target, token });
    }
    if let Some(index) = prefix.rfind("requires=(")
        && !prefix[index + "requires=(".len()..].contains(')')
    {
        let start = index + "requires=(".len();
        return Some(Site::Conditions(token_span(
            key, text, line, line_start, start, cursor,
        )));
    }
    if trimmed.starts_with(":if ") || trimmed.starts_with(":match ") {
        let start = trim_offset + trimmed.find(' ').map_or(trimmed.len(), |index| index + 1);
        return Some(Site::Conditions(token_span(
            key, text, line, line_start, start, cursor,
        )));
    }
    if trimmed.starts_with("!") {
        let start = trim_offset + 1;
        return Some(Site::Effects(token_span(
            key, text, line, line_start, start, cursor,
        )));
    }
    if let Some(index) = prefix.rfind("reason=") {
        let start = index + "reason=".len();
        return Some(Site::AvailabilityReasons(token_span(
            key, text, line, line_start, start, cursor,
        )));
    }
    if let Some(assignment) = recite_parser::metadata_assignment_at(prefix, prefix.len()) {
        let token_start = line_start + assignment.value_start;
        let token = span(key, text, token_start, line_start + cursor);
        if assignment.key == "speaker" && !trimmed.starts_with('?') {
            return Some(Site::Speakers(token));
        }
        return Some(Site::MetadataValue {
            key: assignment.key.to_owned(),
            token,
            target: metadata_target(trimmed),
        });
    }
    let token = prefix.split_whitespace().last().unwrap_or_default();
    let token_start = cursor.saturating_sub(token.len());
    if !token.is_empty()
        && !token.contains('=')
        && matches!(trimmed.as_bytes().first(), Some(b':' | b'>' | b'?'))
        && prefix.split_whitespace().count() >= 2
    {
        return Some(Site::MetadataKey {
            span: span(key, text, line_start + token_start, line_start + cursor),
            target: metadata_target(trimmed),
        });
    }
    None
}

fn metadata_target(line: &str) -> MetadataTarget {
    match line.as_bytes().first() {
        Some(b':') => MetadataTarget::Block,
        Some(b'?') => MetadataTarget::Choice,
        _ => MetadataTarget::Line,
    }
}

pub(super) fn token_at(
    key: &DocumentKey,
    text: &str,
    position: SourcePosition,
) -> Option<(String, SourceSpan)> {
    let (line, line_start) = line_at(text, position.line())?;
    let cursor = byte_at_column(line, position.column());
    let is_token =
        |character: char| character.is_alphanumeric() || matches!(character, '_' | '.' | '-');
    let mut start = cursor;
    while start > 0 {
        let (index, character) = line[..start].char_indices().next_back()?;
        if !is_token(character) {
            break;
        }
        start = index;
    }
    let mut end = cursor;
    while let Some(character) = line.get(end..)?.chars().next() {
        if !is_token(character) {
            break;
        }
        end += character.len_utf8();
    }
    if start == end {
        return None;
    }
    Some((
        line[start..end].to_owned(),
        span(key, text, line_start + start, line_start + end),
    ))
}

fn line_at(text: &str, line_number: u32) -> Option<(&str, usize)> {
    let mut offset = 0;
    for (index, line) in text.split_inclusive('\n').enumerate() {
        if u32::try_from(index + 1).ok()? == line_number {
            let line = line.strip_suffix('\n').unwrap_or(line);
            return Some((line.strip_suffix('\r').unwrap_or(line), offset));
        }
        offset += line.len();
    }
    if u32::try_from(text.split('\n').count()).ok()? == line_number {
        return Some((text.rsplit('\n').next().unwrap_or_default(), offset));
    }
    None
}

fn byte_at_column(line: &str, column: u32) -> usize {
    line.char_indices()
        .nth(column.saturating_sub(1) as usize)
        .map_or(line.len(), |(index, _)| index)
}

fn span(key: &DocumentKey, text: &str, start: usize, end: usize) -> SourceSpan {
    SourceSpan::new(
        key.as_str(),
        position(text, start),
        Some(position(text, end)),
    )
}

fn token_span(
    key: &DocumentKey,
    text: &str,
    line: &str,
    line_start: usize,
    start: usize,
    cursor: usize,
) -> SourceSpan {
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

fn position(text: &str, offset: usize) -> SourcePosition {
    let mut line = 1u32;
    let mut column = 1u32;
    for character in text[..offset.min(text.len())].chars() {
        if character == '\n' {
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
