use recite_core::{ChoiceEcho, LineId, SourceMetadataEntry, SourceSpan};

use crate::header::HeaderKeyValue;

pub(super) fn metadata_entry(kv: HeaderKeyValue<'_>) -> Result<SourceMetadataEntry, SourceSpan> {
    let value = kv.parse_value()?;

    Ok(SourceMetadataEntry::new(kv.key, value)
        .with_source_span(kv.field_span)
        .with_key_value_spans(kv.key_span, Some(kv.value_span)))
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
