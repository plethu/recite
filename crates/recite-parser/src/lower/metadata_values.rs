use recite_core::{ChoiceEcho, LineId, SourceMetadataEntry, SourceSpan};

use crate::header::HeaderKeyValue;

pub(super) fn metadata_entry(kv: HeaderKeyValue<'_>) -> Result<SourceMetadataEntry, SourceSpan> {
    let value = kv.parse_value()?;

    let element_spans = array_element_spans(kv.value, &kv.value_span);
    Ok(SourceMetadataEntry::new(kv.key, value)
        .with_source_span(kv.field_span)
        .with_key_value_spans(kv.key_span, Some(kv.value_span))
        .with_value_element_spans(element_spans))
}

fn array_element_spans(value: &str, value_span: &SourceSpan) -> Vec<SourceSpan> {
    let Some(inner) = value
        .strip_prefix('[')
        .and_then(|value| value.strip_suffix(']'))
    else {
        return Vec::new();
    };
    let mut spans = Vec::new();
    let mut start = 0;
    let mut quote = false;
    for (index, character) in inner.char_indices() {
        match character {
            '"' => quote = !quote,
            ',' if !quote => {
                push_element_span(inner, start, index, value_span, &mut spans);
                start = index + 1;
            }
            _ => {}
        }
    }
    push_element_span(inner, start, inner.len(), value_span, &mut spans);
    spans
}

fn push_element_span(
    inner: &str,
    start: usize,
    end: usize,
    value_span: &SourceSpan,
    spans: &mut Vec<SourceSpan>,
) {
    let trimmed_start = inner[start..end].len() - inner[start..end].trim_start().len();
    let trimmed_end = inner[start..end].trim_end().len() + start;
    let start = start + trimmed_start;
    if start >= trimmed_end {
        return;
    }
    let path = &value_span.file;
    let line = value_span.start.line();
    let column = value_span.start.column() as usize + 1 + inner[..start].chars().count();
    spans.push(crate::source::span_for_text(
        path,
        line,
        column,
        &inner[start..trimmed_end],
    ));
}

pub(super) fn choice_echo(value: &str) -> Option<ChoiceEcho> {
    match value {
        "none" => Some(ChoiceEcho::None),
        "selected_text" => Some(ChoiceEcho::SelectedText),
        _ => {
            let line_id = value.strip_prefix("line(")?.strip_suffix(')')?;
            Some(ChoiceEcho::Line(LineId::new(line_id).ok()?))
        }
    }
}

pub(super) fn is_placeholder_name(value: &str) -> bool {
    let mut chars = value.chars();
    let Some(first) = chars.next() else {
        return false;
    };
    first.is_ascii_lowercase()
        && chars.all(|character| {
            character.is_ascii_lowercase() || character.is_ascii_digit() || character == '_'
        })
}
