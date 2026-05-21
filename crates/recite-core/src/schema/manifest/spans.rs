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

pub(crate) fn key_span(file: &str, source: &str, key: &str) -> SourceSpan {
    string_span(file, source, key).unwrap_or_else(|| document_start_span(file))
}

fn string_span(file: &str, source: &str, needle: &str) -> Option<SourceSpan> {
    let quoted = format!("\"{}\"", escape_json_string(needle));
    let start = source.find(&quoted)?;
    let end = start + quoted.len();
    Some(SourceSpan::new(
        file,
        position_for_offset(source, start),
        Some(position_for_offset(source, end)),
    ))
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

fn escape_json_string(value: &str) -> String {
    value.replace('\\', "\\\\").replace('"', "\\\"")
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
        let quoted = format!("\"{}\"", escape_json_string(needle));
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

        let Some(relative_start) = source[search_start..range.end].find(&quoted) else {
            return document_start_span(file);
        };

        let start = search_start + relative_start;
        let end = start + quoted.len();
        self.next_offsets.insert(search_key, end);

        SourceSpan::new(
            file,
            position_for_offset(source, start),
            Some(position_for_offset(source, end)),
        )
    }
}

#[derive(Clone, Copy, Debug)]
struct SourceRange {
    start: usize,
    end: usize,
}

fn source_section_range(source: &str, section: &str) -> Option<SourceRange> {
    let mut depth = 0_u32;
    let mut in_string = false;
    let mut escaped = false;
    let mut string_start = None;

    for (index, character) in source.char_indices() {
        if in_string {
            if escaped {
                escaped = false;
            } else if character == '\\' {
                escaped = true;
            } else if character == '"' {
                in_string = false;
                if depth == 1 {
                    let start = string_start.expect("string start is recorded before entering");
                    let content_start = start + 1;
                    if source[content_start..index] == *section
                        && next_non_whitespace(source, index + character.len_utf8()) == Some(':')
                    {
                        let value_start =
                            section_value_start(source, index + character.len_utf8())?;
                        let value_end = value_end(source, value_start)?;
                        return Some(SourceRange {
                            start: value_start,
                            end: value_end,
                        });
                    }
                }
            }
            continue;
        }

        if character == '"' {
            in_string = true;
            string_start = Some(index);
        } else if character == '{' || character == '[' {
            depth = depth.saturating_add(1);
        } else if character == '}' || character == ']' {
            depth = depth.saturating_sub(1);
        }
    }

    None
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

fn next_non_whitespace(source: &str, start: usize) -> Option<char> {
    source[start..]
        .chars()
        .find(|character| !character.is_whitespace())
}
