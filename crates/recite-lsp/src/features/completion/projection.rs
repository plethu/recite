use lsp_types::{CompletionItem, CompletionItemKind, CompletionResponse, Documentation};
use recite_core::ProjectSchema;

pub(super) fn schema_json_completion_items(
    text: &str,
    position: lsp_types::Position,
    line_prefix: &str,
    schema: &ProjectSchema,
) -> Option<CompletionResponse> {
    if json_field_value_is_completing(line_prefix, "function") {
        return Some(items(projection_query_function_items(schema)));
    }
    if object_key_is_completing(line_prefix) && completing_projector_key(text, position) {
        return Some(items(presentation_projector_items(schema)));
    }
    let projector_id = current_projector_id(text, position)?;
    let projector = schema.presentation_projectors.get(&projector_id)?;
    if json_field_value_is_completing(line_prefix, "query_result") {
        return Some(items(projector_query_items(projector)));
    }
    if json_field_value_is_completing(line_prefix, "input") {
        return Some(items(projector_input_items(projector)));
    }
    if object_key_is_completing(line_prefix) && completing_output_key(text, position) {
        return Some(items(presentation_output_items(projector)));
    }
    if json_field_value_is_completing(line_prefix, "template_id") {
        return Some(items(presentation_label_template_items(projector)));
    }
    None
}

fn items(items: Vec<CompletionItem>) -> CompletionResponse {
    CompletionResponse::Array(items)
}

fn projection_query_function_items(schema: &ProjectSchema) -> Vec<CompletionItem> {
    schema
        .projection_queries
        .iter()
        .map(|(name, definition)| CompletionItem {
            label: name.clone(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some(format!(
                "projection query -> {}",
                super::super::schema_type_detail(&definition.returns)
            )),
            documentation: Some(Documentation::String(
                "Schema-owned presentation projection query function".to_owned(),
            )),
            ..CompletionItem::default()
        })
        .collect()
}

fn presentation_projector_items(schema: &ProjectSchema) -> Vec<CompletionItem> {
    schema
        .presentation_projectors
        .keys()
        .map(|id| CompletionItem {
            label: id.clone(),
            kind: Some(CompletionItemKind::REFERENCE),
            detail: Some("presentation projector".to_owned()),
            ..CompletionItem::default()
        })
        .collect()
}

fn projector_input_items(
    projector: &recite_core::SchemaPresentationProjectorDefinition,
) -> Vec<CompletionItem> {
    projector
        .inputs
        .iter()
        .map(|input| CompletionItem {
            label: input.name.clone(),
            kind: Some(CompletionItemKind::VARIABLE),
            detail: Some(format!(
                "projection input -> {}",
                super::super::schema_type_detail(&input.type_ref)
            )),
            ..CompletionItem::default()
        })
        .collect()
}

fn projector_query_items(
    projector: &recite_core::SchemaPresentationProjectorDefinition,
) -> Vec<CompletionItem> {
    projector
        .queries
        .iter()
        .map(|(name, query)| CompletionItem {
            label: name.clone(),
            kind: Some(CompletionItemKind::VARIABLE),
            detail: Some(format!("projection query call -> {}", query.function)),
            ..CompletionItem::default()
        })
        .collect()
}

fn presentation_output_items(
    projector: &recite_core::SchemaPresentationProjectorDefinition,
) -> Vec<CompletionItem> {
    projector
        .outputs
        .iter()
        .map(|(id, output)| CompletionItem {
            label: id.clone(),
            kind: Some(CompletionItemKind::VALUE),
            detail: Some(format!("presentation output -> {}", output.kind)),
            ..CompletionItem::default()
        })
        .collect()
}

fn presentation_label_template_items(
    projector: &recite_core::SchemaPresentationProjectorDefinition,
) -> Vec<CompletionItem> {
    projector
        .outputs
        .values()
        .filter_map(|output| output.label.as_ref())
        .map(|label| CompletionItem {
            label: label.template_id.clone(),
            kind: Some(CompletionItemKind::CONSTANT),
            detail: Some("presentation label template".to_owned()),
            documentation: Some(Documentation::String(label.source_text.clone())),
            ..CompletionItem::default()
        })
        .collect()
}

fn json_field_value_is_completing(line_prefix: &str, field: &str) -> bool {
    let Some((_, value_prefix)) = line_prefix.rsplit_once(&format!("\"{field}\"")) else {
        return false;
    };
    let trimmed = value_prefix.trim_start();
    let Some(value) = trimmed.strip_prefix(':') else {
        return false;
    };
    let value = value.trim_start();
    value.starts_with('"') && value[1..].matches('"').count() == 0
}

fn object_key_is_completing(line_prefix: &str) -> bool {
    let trimmed = line_prefix.trim_start();
    trimmed.starts_with('"') && !trimmed[1..].contains('"')
}

fn completing_projector_key(text: &str, position: lsp_types::Position) -> bool {
    let Some(prefix) = text_prefix(text, position) else {
        return false;
    };
    let Some(section_index) = prefix.rfind("\"presentation_projectors\"") else {
        return false;
    };
    let section = &prefix[section_index..];
    let mut scanner = JsonObjectScanner::default();
    scanner.scan(section);
    scanner.in_string
        && scanner.string_start_depth == 1
        && scanner.current_object_is("presentation_projectors", 0)
}

fn completing_output_key(text: &str, position: lsp_types::Position) -> bool {
    let Some(prefix) = text_prefix(text, position) else {
        return false;
    };
    let Some(section_index) = prefix.rfind("\"presentation_projectors\"") else {
        return false;
    };
    let section = &prefix[section_index..];
    let mut scanner = JsonObjectScanner::default();
    scanner.scan(section);
    scanner.in_string && scanner.string_start_depth == 3 && scanner.current_object_is("outputs", 2)
}

fn current_projector_id(text: &str, position: lsp_types::Position) -> Option<String> {
    let prefix = text_prefix(text, position)?;
    let section_index = prefix.rfind("\"presentation_projectors\"")?;
    let section = &prefix[section_index..];
    let mut scanner = JsonObjectScanner::default();
    scanner.scan(section);
    scanner
        .object_entries
        .into_iter()
        .filter_map(|(key, depth)| (depth == 1).then_some(key))
        .next_back()
}

fn text_prefix(text: &str, position: lsp_types::Position) -> Option<&str> {
    let line_index = usize::try_from(position.line).ok()?;
    let mut byte_index = 0;
    for (index, line) in text.split_inclusive('\n').enumerate() {
        if index == line_index {
            let line_without_newline = line.trim_end_matches('\n').trim_end_matches('\r');
            byte_index += byte_index_for_utf16_character(line_without_newline, position.character)?;
            return text.get(..byte_index);
        }
        byte_index += line.len();
    }
    None
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

#[derive(Default)]
struct JsonObjectScanner {
    depth: usize,
    pending_key: Option<(String, usize)>,
    object_entries: Vec<(String, usize)>,
    object_stack: Vec<(String, usize)>,
    in_string: bool,
    escaped: bool,
    string_start_depth: usize,
    string: String,
}

impl JsonObjectScanner {
    fn scan(&mut self, source: &str) {
        for character in source.chars() {
            if self.in_string {
                self.push_string_character(character);
                continue;
            }
            match character {
                '"' => self.start_string(),
                '{' => {
                    if let Some((key, depth)) = self.pending_key.take()
                        && depth == self.depth
                    {
                        self.object_entries.push((key.clone(), self.depth));
                        self.object_stack.push((key, self.depth));
                    }
                    self.depth = self.depth.saturating_add(1);
                }
                '}' => {
                    self.depth = self.depth.saturating_sub(1);
                    while self
                        .object_stack
                        .last()
                        .is_some_and(|(_, depth)| *depth >= self.depth)
                    {
                        self.object_stack.pop();
                    }
                }
                ':' | ',' | '[' | ']' => {}
                character if character.is_whitespace() => {}
                _ => self.pending_key = None,
            }
        }
    }

    fn start_string(&mut self) {
        self.in_string = true;
        self.escaped = false;
        self.string_start_depth = self.depth;
        self.string.clear();
    }

    fn current_object_is(&self, key: &str, depth: usize) -> bool {
        self.object_stack
            .last()
            .is_some_and(|(object_key, object_depth)| object_key == key && *object_depth == depth)
    }

    fn push_string_character(&mut self, character: char) {
        if self.escaped {
            self.string.push(character);
            self.escaped = false;
            return;
        }
        match character {
            '\\' => self.escaped = true,
            '"' => {
                self.in_string = false;
                self.pending_key = Some((self.string.clone(), self.string_start_depth));
            }
            _ => self.string.push(character),
        }
    }
}
