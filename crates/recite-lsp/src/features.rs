use lsp_types::{
    CompletionItem, CompletionItemKind, CompletionResponse, Documentation, Hover, HoverContents,
    MarkupContent, MarkupKind, Position, Range,
};
use recite_core::{ConditionReturnType, ProjectSchema};

const REQUIRES_HOVER: &str =
    "requires=(...) keeps the choice visible and marks it unavailable when the condition is false.";
const IF_HOVER: &str =
    ":if structurally omits hidden dialogue content when the condition is false.";

pub(crate) fn completion(
    text: &str,
    position: Position,
    schema: &ProjectSchema,
) -> Option<CompletionResponse> {
    let line = line_prefix(text, position)?;
    match completion_context(line) {
        CompletionContext::Requires => Some(CompletionResponse::Array(
            schema
                .conditions
                .iter()
                .map(|(name, definition)| CompletionItem {
                    label: name.clone(),
                    kind: Some(CompletionItemKind::FUNCTION),
                    detail: Some(condition_detail(&definition.returns)),
                    documentation: Some(Documentation::String(
                        "Recite condition function".to_owned(),
                    )),
                    ..CompletionItem::default()
                })
                .collect(),
        )),
        CompletionContext::Reason => Some(CompletionResponse::Array(
            schema
                .availability_reasons
                .iter()
                .filter(|(_, definition)| definition.params.is_empty())
                .map(|(id, definition)| CompletionItem {
                    label: id.as_str().to_owned(),
                    kind: Some(CompletionItemKind::CONSTANT),
                    detail: Some("parameterless availability reason".to_owned()),
                    documentation: Some(Documentation::String(definition.template.clone())),
                    ..CompletionItem::default()
                })
                .collect(),
        )),
        CompletionContext::None => None,
    }
}

pub(crate) fn hover(text: &str, position: Position) -> Option<Hover> {
    let line_index = usize::try_from(position.line).ok()?;
    let line = text.lines().nth(line_index)?;
    let byte_index = byte_index_for_utf16_character(line, position.character)?;
    if let Some(range) = find_requires_range(line, line_index, byte_index) {
        return Some(hover_response(REQUIRES_HOVER, range));
    }
    if let Some(range) = find_if_range(line, line_index, byte_index) {
        return Some(hover_response(IF_HOVER, range));
    }

    None
}

enum CompletionContext {
    Requires,
    Reason,
    None,
}

fn completion_context(line_prefix: &str) -> CompletionContext {
    if let Some(index) = line_prefix.rfind("requires=(")
        && !line_prefix[index + "requires=(".len()..].contains(')')
    {
        return CompletionContext::Requires;
    }

    if let Some(index) = line_prefix.rfind("reason=") {
        let value = &line_prefix[index + "reason=".len()..];
        if !value.chars().any(char::is_whitespace) {
            return CompletionContext::Reason;
        }
    }

    CompletionContext::None
}

fn line_prefix(text: &str, position: Position) -> Option<&str> {
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

fn condition_detail(return_type: &ConditionReturnType) -> String {
    match return_type {
        ConditionReturnType::Bool => "condition -> bool".to_owned(),
        ConditionReturnType::Enum(name) => format!("condition -> enum:{name}"),
    }
}
