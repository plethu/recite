use lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position, Range};

pub(super) fn find_requires_range(
    line: &str,
    line_index: usize,
    byte_index: usize,
) -> Option<Range> {
    let start = line.find("requires=(")?;
    let end = match line[start..].find(')') {
        Some(relative_end) => start + relative_end + 1,
        None => line.len(),
    };
    (start <= byte_index && byte_index <= end).then(|| range(line, line_index, start, end))
}

pub(super) fn find_if_range(line: &str, line_index: usize, byte_index: usize) -> Option<Range> {
    let start = line.find(":if")?;
    let end = start + ":if".len();
    (start <= byte_index && byte_index <= end).then(|| range(line, line_index, start, end))
}

pub(super) fn word_at(line: &str, line_index: usize, byte_index: usize) -> Option<(&str, Range)> {
    if byte_index > line.len() {
        return None;
    }
    let mut start = byte_index;
    for (index, character) in line[..byte_index].char_indices().rev() {
        if !is_symbol_character(character) {
            break;
        }
        start = index;
    }
    let mut end = byte_index;
    for (relative_index, character) in line[byte_index..].char_indices() {
        if !is_symbol_character(character) {
            break;
        }
        end = byte_index + relative_index + character.len_utf8();
    }
    (start < end).then(|| (&line[start..end], range(line, line_index, start, end)))
}

fn is_symbol_character(character: char) -> bool {
    character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | ':')
}

pub(super) fn metadata_value_key_at(line: &str, byte_index: usize) -> Option<&str> {
    let trimmed = line.trim_start();
    if !(trimmed.starts_with("::") || trimmed.starts_with('>') || trimmed.starts_with('?')) {
        return None;
    }
    if byte_index > line.len() {
        return None;
    }
    let token_start = line[..byte_index]
        .rfind(char::is_whitespace)
        .map_or(0, |index| index + 1);
    let token_end = line[byte_index..]
        .find(char::is_whitespace)
        .map_or(line.len(), |index| byte_index + index);
    let token = line.get(token_start..token_end)?;
    let (key, value) = token.split_once('=')?;
    let value_start = token_start + key.len() + 1;
    let first = value.as_bytes().first().copied()?;
    (key != "speaker"
        && !key.is_empty()
        && !value.is_empty()
        && (first.is_ascii_alphabetic() || first == b'_')
        && byte_index >= value_start)
        .then_some(key)
}

pub(super) fn hover_response(value: &str, range: Range) -> Hover {
    Hover {
        contents: HoverContents::Markup(MarkupContent {
            kind: MarkupKind::PlainText,
            value: value.to_owned(),
        }),
        range: Some(range),
    }
}

fn range(text_line: &str, line: usize, start: usize, end: usize) -> Range {
    Range {
        start: Position {
            line: u32::try_from(line).unwrap_or(u32::MAX),
            character: utf16_units_for_byte_index(text_line, start),
        },
        end: Position {
            line: u32::try_from(line).unwrap_or(u32::MAX),
            character: utf16_units_for_byte_index(text_line, end),
        },
    }
}

pub(crate) fn byte_index_for_utf16_character(line: &str, character: u32) -> Option<usize> {
    let mut utf16_units = 0_u32;
    for (byte_index, value) in line.char_indices() {
        if utf16_units == character {
            return Some(byte_index);
        }
        utf16_units = utf16_units.saturating_add(value.len_utf16() as u32);
        if utf16_units > character {
            return Some(byte_index);
        }
    }
    (utf16_units == character).then_some(line.len())
}

fn utf16_units_for_byte_index(line: &str, byte_index: usize) -> u32 {
    line.get(..byte_index)
        .unwrap_or(line)
        .chars()
        .map(char::len_utf16)
        .fold(0_u32, |total, width| {
            total.saturating_add(u32::try_from(width).unwrap_or(u32::MAX))
        })
}
