use std::collections::BTreeMap;

use crate::{SourcePosition, SourceSpan};

pub(crate) fn json_error_span(file: &str, error: &serde_json::Error) -> SourceSpan {
    let line = u32::try_from(error.line()).unwrap_or(u32::MAX).max(1);
    let column = u32::try_from(error.column()).unwrap_or(u32::MAX).max(1);
    SourceSpan::point(
        file,
        SourcePosition::new(line, column).expect("line and column are clamped to non-zero"),
    )
}

pub(crate) fn top_level_key_span(file: &str, source: &str, key: &str) -> SourceSpan {
    top_level_key_range(source, key)
        .map(|range| {
            SourceSpan::new(
                file,
                position_for_offset(source, range.start),
                Some(position_for_offset(source, range.end)),
            )
        })
        .unwrap_or_else(|| document_start_span(file))
}

pub(crate) fn top_level_number_token<'a>(source: &'a str, key: &str) -> Option<&'a str> {
    let range = top_level_key_range(source, key)?;
    let colon = skip_whitespace(source, range.end)?;
    if source.as_bytes().get(colon) != Some(&b':') {
        return None;
    }
    let value_start = skip_whitespace(source, colon + 1)?;
    Some(json_number_token(source, value_start))
}

fn document_start_span(file: &str) -> SourceSpan {
    SourceSpan::point(
        file,
        SourcePosition::new(1, 1).expect("static source position is valid"),
    )
}

fn position_for_offset(source: &str, offset: usize) -> SourcePosition {
    let mut line = 1_u32;
    let mut column = 1_u32;
    for (index, character) in source.char_indices() {
        if index >= offset {
            break;
        }
        if character == '\n' {
            line = line.saturating_add(1);
            column = 1;
        } else {
            column = column.saturating_add(1);
        }
    }
    SourcePosition::new(line, column).expect("line and column start at one")
}

fn top_level_key_range(source: &str, key: &str) -> Option<SourceRange> {
    let bytes = source.as_bytes();
    let mut depth = 0usize;
    let mut index = 0usize;

    while index < bytes.len() {
        match bytes[index] {
            b'"' => {
                let string_start = index;
                let string_end = skip_json_string(bytes, string_start)?;
                if depth == 1 && json_string_equals(source, string_start, string_end, key) {
                    return Some(SourceRange {
                        start: string_start,
                        end: string_end + 1,
                    });
                }
                index = string_end + 1;
            }
            b'{' | b'[' => {
                depth += 1;
                index += 1;
            }
            b'}' | b']' => {
                depth = depth.checked_sub(1)?;
                index += 1;
            }
            _ => index += 1,
        }
    }

    None
}

fn skip_json_string(bytes: &[u8], start: usize) -> Option<usize> {
    let mut escaped = false;
    let mut index = start + 1;
    while index < bytes.len() {
        match bytes[index] {
            b'\\' if !escaped => escaped = true,
            b'"' if !escaped => return Some(index),
            _ => escaped = false,
        }
        index += 1;
    }

    None
}

fn json_string_equals(source: &str, start: usize, end: usize, expected: &str) -> bool {
    serde_json::from_str::<String>(&source[start..=end]).is_ok_and(|value| value == expected)
}

fn skip_whitespace(source: &str, start: usize) -> Option<usize> {
    source[start..]
        .find(|character: char| !character.is_whitespace())
        .map(|offset| start + offset)
}

fn json_number_token(source: &str, value_start: usize) -> &str {
    let value_end = source[value_start..]
        .find(|character: char| character.is_whitespace() || character == ',' || character == '}')
        .map_or(source.len(), |offset| value_start + offset);

    &source[value_start..value_end]
}

#[derive(Debug, Default)]
pub(crate) struct ManifestSpans {
    next_offsets: BTreeMap<String, usize>,
    active_range: Option<SourceRange>,
}

impl ManifestSpans {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn enter_section(&mut self, source: &str, section: &str) {
        self.active_range = source_section_range(source, section);
        self.next_offsets.clear();
    }

    pub(crate) fn next_string_span(
        &mut self,
        file: &str,
        source: &str,
        needle: &str,
    ) -> SourceSpan {
        let range = self.active_range.unwrap_or(SourceRange {
            start: 0,
            end: source.len(),
        });
        let search_key = format!("{}:{needle}", range.start);
        let search_start = self
            .next_offsets
            .get(&search_key)
            .copied()
            .unwrap_or(range.start);
        if search_start > range.end {
            return document_start_span(file);
        }

        let Some(span_range) = next_json_string_range(source, range, search_start, needle) else {
            return document_start_span(file);
        };

        self.next_offsets.insert(search_key, span_range.end);

        SourceSpan::new(
            file,
            position_for_offset(source, span_range.start),
            Some(position_for_offset(source, span_range.end)),
        )
    }
}

#[derive(Clone, Copy, Debug)]
struct SourceRange {
    start: usize,
    end: usize,
}

fn next_json_string_range(
    source: &str,
    range: SourceRange,
    search_start: usize,
    needle: &str,
) -> Option<SourceRange> {
    let bytes = source.as_bytes();
    let mut index = search_start;

    while index < range.end {
        if bytes[index] != b'"' {
            index += 1;
            continue;
        }

        let string_start = index;
        let string_end = skip_json_string(bytes, string_start)?;
        if string_end >= range.end {
            return None;
        }

        if json_string_equals(source, string_start, string_end, needle) {
            return Some(SourceRange {
                start: string_start,
                end: string_end + 1,
            });
        }

        index = string_end + 1;
    }

    None
}

fn source_section_range(source: &str, section: &str) -> Option<SourceRange> {
    let key_range = top_level_key_range(source, section)?;
    let value_start = section_value_start(source, key_range.end)?;
    let value_end = value_end(source, value_start)?;
    Some(SourceRange {
        start: value_start,
        end: value_end,
    })
}

fn value_end(source: &str, start: usize) -> Option<usize> {
    let opening = source[start..].chars().next()?;
    let closing = match opening {
        '{' => '}',
        '[' => ']',
        _ => return Some(start + opening.len_utf8()),
    };
    let mut depth = 0_u32;
    let mut in_string = false;
    let mut escaped = false;

    for (relative_index, character) in source[start..].char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
            }
            continue;
        }

        if character == '"' {
            in_string = true;
        } else if character == opening {
            depth = depth.saturating_add(1);
        } else if character == closing {
            depth = depth.saturating_sub(1);
            if depth == 0 {
                return Some(start + relative_index + character.len_utf8());
            }
        }
    }

    None
}

fn section_value_start(source: &str, key_end: usize) -> Option<usize> {
    let colon = source[key_end..]
        .find(':')
        .map(|offset| key_end + offset + 1)?;
    source[colon..]
        .find(|character: char| !character.is_whitespace())
        .map(|offset| colon + offset)
}
