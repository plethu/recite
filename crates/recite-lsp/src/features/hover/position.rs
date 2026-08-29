use lsp_types::{Hover, HoverContents, MarkupContent, MarkupKind, Position, Range};
use recite_core::ProjectSchema;
use recite_core::{SourceMetadataScalar, SourceMetadataValue};
use recite_ui::UiCatalog;

use super::super::context::selector_site;
use super::super::schema_hover::{AuthoringPosition, SchemaValueHover, schema_value_hover};

pub(super) enum ValuePosition<'a> {
    Metadata {
        key: &'a str,
        value: &'a str,
        complete_symbol: bool,
    },
    DedicatedChoiceClause {
        complete_symbol: bool,
    },
}

pub(super) enum MetadataHover {
    NotMetadataPosition,
    DedicatedChoiceClause,
    Resolved(Hover),
    Invalid,
}

pub(super) struct MetadataHoverInput<'a> {
    pub(super) text: &'a str,
    pub(super) line: &'a str,
    pub(super) line_index: usize,
    pub(super) byte_index: usize,
    pub(super) word: &'a str,
    pub(super) range: Range,
    pub(super) schema: &'a ProjectSchema,
    pub(super) catalog: &'a UiCatalog,
}

pub(super) fn metadata_hover(input: MetadataHoverInput<'_>) -> MetadataHover {
    let MetadataHoverInput {
        text,
        line,
        line_index,
        byte_index,
        word,
        range,
        schema,
        catalog,
    } = input;
    let Some(value_position) = value_position_at(line, byte_index) else {
        return MetadataHover::NotMetadataPosition;
    };
    let (key, value, complete_symbol) = match value_position {
        ValuePosition::Metadata {
            key,
            value,
            complete_symbol,
        } => (key, value, complete_symbol),
        ValuePosition::DedicatedChoiceClause { complete_symbol } => {
            return if complete_symbol {
                MetadataHover::DedicatedChoiceClause
            } else {
                MetadataHover::Invalid
            };
        }
    };
    let Some(site) = selector_site(line) else {
        return MetadataHover::NotMetadataPosition;
    };
    if !complete_symbol {
        return MetadataHover::Invalid;
    }
    match schema_value_hover(
        schema,
        key,
        word,
        value,
        &AuthoringPosition {
            text,
            line_index,
            line,
            site,
        },
        catalog,
    ) {
        SchemaValueHover::Resolved(value) => MetadataHover::Resolved(hover_response(&value, range)),
        SchemaValueHover::Invalid => MetadataHover::Invalid,
    }
}

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
    character.is_ascii_alphanumeric() || matches!(character, '_' | '-' | ':' | '.')
}

pub(super) fn value_position_at(line: &str, byte_index: usize) -> Option<ValuePosition<'_>> {
    let trimmed = line.trim_start();
    if !(trimmed.starts_with("::") || trimmed.starts_with('>') || trimmed.starts_with('?')) {
        return None;
    }
    if byte_index > line.len() {
        return None;
    }
    let assignment = recite_parser::metadata_assignment_at(line, byte_index)?;
    if assignment.key.is_empty() || byte_index < assignment.value_start {
        return None;
    }
    if trimmed.starts_with('?') && assignment.key == "reason" {
        return Some(ValuePosition::DedicatedChoiceClause {
            complete_symbol: is_symbol(assignment.value),
        });
    }
    Some(ValuePosition::Metadata {
        key: assignment.key,
        value: assignment.value,
        complete_symbol: is_complete_symbol_value(assignment.value),
    })
}

fn is_complete_symbol_value(value: &str) -> bool {
    if is_symbol(value) {
        return true;
    }
    let Some(SourceMetadataValue::Array(values)) = recite_parser::parse_metadata_value(value)
    else {
        return false;
    };
    !values.is_empty()
        && values
            .iter()
            .all(|value| matches!(value, SourceMetadataScalar::Symbol(_)))
}

fn is_symbol(value: &str) -> bool {
    let mut characters = value.chars();
    let Some(first) = characters.next() else {
        return false;
    };
    (first.is_ascii_alphabetic() || first == '_')
        && characters.all(|character| {
            character.is_ascii_alphanumeric() || matches!(character, '_' | '.' | '-')
        })
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
