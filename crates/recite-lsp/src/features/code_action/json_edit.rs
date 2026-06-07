use lsp_types::{Range, TextEdit};
use serde_json::Value;

use super::newline_for;

pub(super) fn object_section_entry(
    text: &str,
    section: &str,
    name: &str,
    body: &str,
) -> Option<TextEdit> {
    let manifest = serde_json::from_str::<Value>(text).ok()?;
    let section_value = manifest.as_object()?.get(section)?;
    let section_object = section_value.as_object()?;
    if section_object.contains_key(name) {
        return None;
    }

    let section_range = find_top_level_object_section(text, section)?;
    let position = byte_position(text, section_range.close_brace);
    let newline = newline_for(text);
    let comma = if section_object.is_empty() { "" } else { "," };
    Some(TextEdit {
        range: Range {
            start: position,
            end: position,
        },
        new_text: format!(
            "{comma}{newline}    {}: {body}{newline}  ",
            json_string(name)
        ),
    })
}

struct JsonObjectRange {
    close_brace: usize,
}

fn find_top_level_object_section(text: &str, section: &str) -> Option<JsonObjectRange> {
    let bytes = text.as_bytes();
    let mut index = 0;
    let mut depth = 0_usize;
    while index < bytes.len() {
        match bytes[index] {
            b'"' => {
                let string_start = index;
                let value = parse_json_string_literal(text, &mut index)?;
                if depth == 1 && value == section {
                    let mut cursor = skip_json_ws(bytes, index);
                    if bytes.get(cursor) != Some(&b':') {
                        continue;
                    }
                    cursor = skip_json_ws(bytes, cursor.saturating_add(1));
                    if bytes.get(cursor) != Some(&b'{') {
                        return None;
                    }
                    let close_brace = matching_json_object_close(text, cursor)?;
                    return Some(JsonObjectRange { close_brace });
                }
                if index <= string_start {
                    return None;
                }
            }
            b'{' => {
                depth = depth.saturating_add(1);
                index = index.saturating_add(1);
            }
            b'}' => {
                depth = depth.saturating_sub(1);
                index = index.saturating_add(1);
            }
            _ => {
                index = index.saturating_add(1);
            }
        }
    }
    None
}

fn matching_json_object_close(text: &str, open_brace: usize) -> Option<usize> {
    let bytes = text.as_bytes();
    let mut index = open_brace;
    let mut depth = 0_usize;
    while index < bytes.len() {
        match bytes[index] {
            b'"' => {
                parse_json_string_literal(text, &mut index)?;
            }
            b'{' => {
                depth = depth.saturating_add(1);
                index = index.saturating_add(1);
            }
            b'}' => {
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    return Some(index);
                }
                index = index.saturating_add(1);
            }
            _ => {
                index = index.saturating_add(1);
            }
        }
    }
    None
}

fn parse_json_string_literal(text: &str, index: &mut usize) -> Option<String> {
    let bytes = text.as_bytes();
    if bytes.get(*index) != Some(&b'"') {
        return None;
    }
    let start = *index;
    *index = index.saturating_add(1);
    while *index < bytes.len() {
        match bytes[*index] {
            b'\\' => {
                *index = index.saturating_add(2);
            }
            b'"' => {
                *index = index.saturating_add(1);
                return serde_json::from_str(&text[start..*index]).ok();
            }
            _ => {
                *index = index.saturating_add(1);
            }
        }
    }
    None
}

fn skip_json_ws(bytes: &[u8], mut index: usize) -> usize {
    while matches!(bytes.get(index), Some(b' ' | b'\n' | b'\r' | b'\t')) {
        index = index.saturating_add(1);
    }
    index
}

pub(super) fn json_string(value: &str) -> String {
    serde_json::to_string(value).unwrap_or_else(|_| "\"\"".to_owned())
}

pub(super) fn byte_position(text: &str, offset: usize) -> lsp_types::Position {
    let mut line = 0_u32;
    let mut line_start = 0_usize;
    for (index, character) in text.char_indices() {
        if index >= offset {
            break;
        }
        if character == '\n' {
            line = line.saturating_add(1);
            line_start = index.saturating_add(1);
        }
    }
    let capped = offset.min(text.len());
    let character = text[line_start..capped]
        .chars()
        .map(char::len_utf16)
        .fold(0_u32, |total, width| {
            total.saturating_add(u32::try_from(width).unwrap_or(u32::MAX))
        });
    lsp_types::Position { line, character }
}
