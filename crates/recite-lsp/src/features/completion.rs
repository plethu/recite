use std::collections::BTreeSet;

use lsp_types::{CompletionItem, CompletionItemKind, CompletionResponse, Documentation, Position};
use recite_core::{
    MetadataContextSelector, MetadataDomainDefinition, MissingMetadataContextPolicy, ProjectSchema,
};

use crate::workspace::LiveProjectSnapshot;

pub(super) fn completion(
    text: &str,
    position: Position,
    schema: &ProjectSchema,
    snapshot: &LiveProjectSnapshot,
) -> Option<CompletionResponse> {
    let line = super::line_prefix(text, position)?;
    match completion_context(line) {
        CompletionContext::BlockReference => Some(items(block_completion_items(snapshot))),
        CompletionContext::Speaker => Some(items(speaker_completion_items(schema))),
        CompletionContext::MetadataKey => Some(items(metadata_key_completion_items(schema))),
        CompletionContext::MetadataValue { key, header_kind } => {
            let line_index = usize::try_from(position.line).ok()?;
            let line_text = text.lines().nth(line_index)?;
            Some(items(metadata_value_completion_items(
                schema,
                text,
                line_text,
                line_index,
                &key,
                header_kind,
            )))
        }
        CompletionContext::Condition => Some(items(condition_completion_items(schema))),
        CompletionContext::Effect => Some(items(effect_completion_items(schema))),
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

enum CompletionContext {
    BlockReference,
    Speaker,
    MetadataKey,
    MetadataValue {
        key: String,
        header_kind: HeaderKind,
    },
    Condition,
    Effect,
    Reason,
    None,
}

#[derive(Clone, Copy)]
enum HeaderKind {
    Block,
    Line,
    Choice,
}

enum SelectorContext {
    Value(String),
    Missing,
    Malformed,
}

fn completion_context(line_prefix: &str) -> CompletionContext {
    if line_prefix.trim_start().starts_with("->") {
        return CompletionContext::BlockReference;
    }

    if let Some(index) = line_prefix.rfind("requires=(")
        && !line_prefix[index + "requires=(".len()..].contains(')')
    {
        return CompletionContext::Condition;
    }

    if line_prefix.trim_start().starts_with(":if ") {
        return CompletionContext::Condition;
    }

    if effect_prefix_is_completing_function(line_prefix) {
        return CompletionContext::Effect;
    }

    if let Some(index) = line_prefix.rfind("reason=") {
        let value = &line_prefix[index + "reason=".len()..];
        if !value.chars().any(char::is_whitespace) {
            return CompletionContext::Reason;
        }
    }

    let header_kind = header_kind(line_prefix);
    if let Some(token) = current_token(line_prefix) {
        if token.starts_with("speaker=") && header_kind.is_some() {
            return CompletionContext::Speaker;
        }
        if let Some((key, _)) = token.split_once('=')
            && !key.is_empty()
            && let Some(header_kind) = header_kind
        {
            return CompletionContext::MetadataValue {
                key: key.to_owned(),
                header_kind,
            };
        }
    }

    if is_metadata_key_position(line_prefix, header_kind) {
        return CompletionContext::MetadataKey;
    }

    CompletionContext::None
}

fn items(items: Vec<CompletionItem>) -> CompletionResponse {
    CompletionResponse::Array(items)
}

fn block_completion_items(snapshot: &LiveProjectSnapshot) -> Vec<CompletionItem> {
    super::block_names(snapshot)
        .into_iter()
        .map(|name| CompletionItem {
            label: name,
            kind: Some(CompletionItemKind::REFERENCE),
            detail: Some("Recite block".to_owned()),
            ..CompletionItem::default()
        })
        .collect()
}

fn speaker_completion_items(schema: &ProjectSchema) -> Vec<CompletionItem> {
    schema
        .speakers
        .iter()
        .map(|(id, definition)| CompletionItem {
            label: id.clone(),
            kind: Some(CompletionItemKind::CONSTANT),
            detail: Some("Recite speaker".to_owned()),
            documentation: definition
                .display_name
                .as_ref()
                .map(|display_name| Documentation::String(display_name.clone())),
            ..CompletionItem::default()
        })
        .collect()
}

fn metadata_key_completion_items(schema: &ProjectSchema) -> Vec<CompletionItem> {
    schema
        .metadata
        .iter()
        .map(|(key, definition)| CompletionItem {
            label: key.clone(),
            kind: Some(CompletionItemKind::FIELD),
            detail: Some(definition.domain.as_ref().map_or_else(
                || "Recite metadata key".to_owned(),
                |domain| format!("Recite metadata key -> {domain}"),
            )),
            ..CompletionItem::default()
        })
        .collect()
}

fn metadata_value_completion_items(
    schema: &ProjectSchema,
    text: &str,
    line: &str,
    line_index: usize,
    key: &str,
    header_kind: HeaderKind,
) -> Vec<CompletionItem> {
    if key == "speaker" {
        return speaker_completion_items(schema);
    }
    let Some(metadata) = schema.metadata.get(key) else {
        return Vec::new();
    };
    let Some(domain_name) = &metadata.domain else {
        return Vec::new();
    };
    metadata_domain_values(schema, domain_name, text, line, line_index, header_kind)
        .into_iter()
        .map(|value| CompletionItem {
            label: value,
            kind: Some(CompletionItemKind::VALUE),
            detail: Some(format!("metadata domain `{domain_name}`")),
            ..CompletionItem::default()
        })
        .collect()
}

fn metadata_domain_values(
    schema: &ProjectSchema,
    domain_name: &str,
    text: &str,
    line: &str,
    line_index: usize,
    header_kind: HeaderKind,
) -> BTreeSet<String> {
    let Some(domain) = schema.metadata_domains.get(domain_name) else {
        return BTreeSet::new();
    };
    match domain {
        MetadataDomainDefinition::Flat(domain) => domain.values.clone(),
        MetadataDomainDefinition::Contextual(domain) => {
            match metadata_domain_context(&domain.selector, text, line, line_index, header_kind) {
                SelectorContext::Value(context) => domain
                    .values_by_context
                    .get(context.as_str())
                    .cloned()
                    .unwrap_or_default(),
                SelectorContext::Missing => missing_context_values(schema, &domain.missing_context),
                SelectorContext::Malformed => BTreeSet::new(),
            }
        }
    }
}

fn missing_context_values(
    schema: &ProjectSchema,
    policy: &MissingMetadataContextPolicy,
) -> BTreeSet<String> {
    match policy {
        MissingMetadataContextPolicy::Diagnostic | MissingMetadataContextPolicy::Empty => {
            BTreeSet::new()
        }
        MissingMetadataContextPolicy::Fallback { domain } => {
            match schema.metadata_domains.get(domain) {
                Some(MetadataDomainDefinition::Flat(domain)) => domain.values.clone(),
                _ => BTreeSet::new(),
            }
        }
    }
}

fn metadata_domain_context(
    selector: &MetadataContextSelector,
    text: &str,
    line: &str,
    line_index: usize,
    header_kind: HeaderKind,
) -> SelectorContext {
    match selector {
        MetadataContextSelector::FieldSpeaker => match header_kind {
            HeaderKind::Line => metadata_symbol(line, "speaker")
                .or_else(|| block_default_speaker(text, line_index))
                .map_or(SelectorContext::Missing, SelectorContext::Value),
            HeaderKind::Block | HeaderKind::Choice => SelectorContext::Missing,
        },
        MetadataContextSelector::MetadataKey(key) => {
            let matches = metadata_selector_values(line, key);
            match matches.as_slice() {
                [] => SelectorContext::Missing,
                [Some(value)] => SelectorContext::Value(value.clone()),
                [_] | [_, ..] => SelectorContext::Malformed,
            }
        }
    }
}

fn condition_completion_items(schema: &ProjectSchema) -> Vec<CompletionItem> {
    schema
        .conditions
        .iter()
        .map(|(name, definition)| CompletionItem {
            label: name.clone(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some(super::condition_detail(&definition.returns)),
            documentation: Some(Documentation::String(
                "Recite condition function".to_owned(),
            )),
            ..CompletionItem::default()
        })
        .collect()
}

fn effect_completion_items(schema: &ProjectSchema) -> Vec<CompletionItem> {
    schema
        .effects
        .iter()
        .map(|(name, definition)| CompletionItem {
            label: name.clone(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some(super::effect_detail(&definition.modes)),
            documentation: Some(Documentation::String("Recite effect request".to_owned())),
            ..CompletionItem::default()
        })
        .collect()
}

fn current_token(line_prefix: &str) -> Option<&str> {
    line_prefix.split_whitespace().last()
}

fn header_kind(line_prefix: &str) -> Option<HeaderKind> {
    let trimmed = line_prefix.trim_start();
    if trimmed.starts_with("::") {
        Some(HeaderKind::Block)
    } else if trimmed.starts_with('>') {
        Some(HeaderKind::Line)
    } else if trimmed.starts_with('?') {
        Some(HeaderKind::Choice)
    } else {
        None
    }
}

fn is_metadata_key_position(line_prefix: &str, header_kind: Option<HeaderKind>) -> bool {
    let Some(header_kind) = header_kind else {
        return false;
    };
    let Some(token) = current_token(line_prefix) else {
        return false;
    };
    if token.is_empty() || token.contains('=') {
        return false;
    }
    let field_count = line_prefix.split_whitespace().count();
    match header_kind {
        HeaderKind::Block => field_count >= 3 && !("default".starts_with(token)),
        HeaderKind::Line | HeaderKind::Choice => field_count >= 3,
    }
}

fn effect_prefix_is_completing_function(line_prefix: &str) -> bool {
    let mut parts = line_prefix.split_whitespace();
    matches!(parts.next(), Some("!"))
        && parts.next().is_some()
        && parts.next().is_none_or(|function| !function.contains('('))
}

fn metadata_symbol(line: &str, key: &str) -> Option<String> {
    let mut values = metadata_selector_values(line, key);
    if values.len() == 1 {
        values.remove(0)
    } else {
        None
    }
}

fn metadata_selector_values(line: &str, key: &str) -> Vec<Option<String>> {
    line.split_whitespace()
        .filter_map(|token| token.split_once('='))
        .filter(|(candidate, _)| *candidate == key)
        .map(|(_, value)| scalar_symbol(value))
        .collect()
}

fn scalar_symbol(value: &str) -> Option<String> {
    let value = value
        .trim_end_matches(',')
        .trim_end_matches(')')
        .trim_end_matches(']');
    (!value.is_empty()
        && value
            .chars()
            .all(|character| character.is_ascii_alphanumeric() || matches!(character, '_' | '-')))
    .then(|| value.to_owned())
}

fn block_default_speaker(text: &str, line_index: usize) -> Option<String> {
    text.lines()
        .take(line_index.saturating_add(1))
        .collect::<Vec<_>>()
        .into_iter()
        .rev()
        .find(|line| line.trim_start().starts_with("::"))
        .and_then(|line| metadata_symbol(line, "speaker"))
}
