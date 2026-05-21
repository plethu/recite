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
}

impl ManifestSpans {
    pub(crate) fn new() -> Self {
        Self::default()
    }

    pub(crate) fn next_string_span(
        &mut self,
        file: &str,
        source: &str,
        needle: &str,
    ) -> SourceSpan {
        let quoted = format!("\"{}\"", escape_json_string(needle));
        let search_start = self.next_offsets.get(needle).copied().unwrap_or(0);
        let Some(relative_start) = source[search_start..].find(&quoted) else {
            return document_start_span(file);
        };

        let start = search_start + relative_start;
        let end = start + quoted.len();
        self.next_offsets.insert(needle.to_owned(), end);

        SourceSpan::new(
            file,
            position_for_offset(source, start),
            Some(position_for_offset(source, end)),
        )
    }
}
