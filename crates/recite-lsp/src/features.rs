use std::collections::BTreeSet;

use lsp_types::{
    CodeActionParams, CodeActionResponse, CompletionResponse, Hover, HoverContents, MarkupContent,
    MarkupKind, Position, Range,
};
use recite_core::{ConditionReturnType, EffectMode, ProjectSchema};

use crate::workspace::LiveProjectSnapshot;

mod code_action;
mod completion;
mod navigation;

const REQUIRES_HOVER: &str =
    "requires=(...) keeps the choice visible and marks it unavailable when the condition is false.";
const IF_HOVER: &str =
    ":if structurally omits hidden dialogue content when the condition is false.";

pub(crate) fn completion(
    text: &str,
    position: Position,
    schema: &ProjectSchema,
    snapshot: &LiveProjectSnapshot,
) -> Option<CompletionResponse> {
    completion::completion(text, position, schema, snapshot)
}

pub(crate) use code_action::CodeActionDocument;

pub(crate) fn code_action(
    params: &CodeActionParams,
    documents: &[CodeActionDocument<'_>],
) -> Option<CodeActionResponse> {
    code_action::code_action(params, documents)
}

pub(crate) use navigation::{NavigationDocument, definition, prepare_rename, references, rename};

pub(crate) fn hover(
    text: &str,
    position: Position,
    schema: Option<&ProjectSchema>,
    snapshot: &LiveProjectSnapshot,
) -> Option<Hover> {
    let line_index = usize::try_from(position.line).ok()?;
    let line = text.lines().nth(line_index)?;
    let byte_index = byte_index_for_utf16_character(line, position.character)?;
    if let Some(range) = find_requires_range(line, line_index, byte_index) {
        return Some(hover_response(REQUIRES_HOVER, range));
    }
    if let Some(range) = find_if_range(line, line_index, byte_index) {
        return Some(hover_response(IF_HOVER, range));
    }

    let (word, range) = word_at(line, line_index, byte_index)?;
    if let Some(schema) = schema {
        if let Some(definition) = schema.speakers.get(word) {
            let value = definition.display_name.as_ref().map_or_else(
                || format!("Recite speaker `{word}`."),
                |display_name| format!("Recite speaker `{word}` ({display_name})."),
            );
            return Some(hover_response(&value, range));
        }
        if let Some(definition) = schema.metadata.get(word) {
            let mut value = format!("Recite metadata key `{word}`.");
            if let Some(domain) = &definition.domain {
                value.push_str(&format!(" Values use metadata domain `{domain}`."));
            }
            return Some(hover_response(&value, range));
        }
        if let Some(definition) = schema.conditions.get(word) {
            return Some(hover_response(
                &condition_detail(&definition.returns),
                range,
            ));
        }
        if let Some(definition) = schema.effects.get(word) {
            return Some(hover_response(&effect_detail(&definition.modes), range));
        }
    }
    if block_names(snapshot).contains(word) {
        return Some(hover_response(
            &format!("Recite block `{word}` in the current project index."),
            range,
        ));
    }

    None
}

pub(super) fn block_names(snapshot: &LiveProjectSnapshot) -> BTreeSet<String> {
    snapshot
        .summaries()
        .iter()
        .flat_map(|summary| summary.blocks.iter().map(|block| block.name.clone()))
        .collect()
}

pub(super) fn line_prefix(text: &str, position: Position) -> Option<&str> {
    let line = text.lines().nth(usize::try_from(position.line).ok()?)?;
    let end = byte_index_for_utf16_character(line, position.character)?;
    line.get(..end)
}

fn find_requires_range(line: &str, line_index: usize, byte_index: usize) -> Option<Range> {
    let start = line.find("requires=(")?;
    let end = match line[start..].find(')') {
        Some(relative_end) => start + relative_end + 1,
        None => line.len(),
    };
    (start <= byte_index && byte_index <= end).then(|| range(line, line_index, start, end))
}

fn find_if_range(line: &str, line_index: usize, byte_index: usize) -> Option<Range> {
    let start = line.find(":if")?;
    let end = start + ":if".len();
    (start <= byte_index && byte_index <= end).then(|| range(line, line_index, start, end))
}

fn word_at(line: &str, line_index: usize, byte_index: usize) -> Option<(&str, Range)> {
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

fn hover_response(value: &str, range: Range) -> Hover {
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

fn byte_index_for_utf16_character(line: &str, character: u32) -> Option<usize> {
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

pub(super) fn condition_detail(return_type: &ConditionReturnType) -> String {
    match return_type {
        ConditionReturnType::Bool => "condition -> bool".to_owned(),
        ConditionReturnType::Enum(name) => format!("condition -> enum:{name}"),
    }
}

pub(super) fn effect_detail(modes: &BTreeSet<EffectMode>) -> String {
    let modes = modes
        .iter()
        .map(|mode| match mode {
            EffectMode::Immediate => "immediate",
            EffectMode::Deferred => "deferred",
            EffectMode::Blocking => "blocking",
        })
        .collect::<Vec<_>>()
        .join(", ");
    format!("effect request -> {modes}")
}
