use lsp_types::{CompletionItem, CompletionItemKind, CompletionResponse, Documentation, Position};
use recite_core::{ProjectSchema, SchemaTypeRef};
use recite_ui::{MsgId, UiCatalog};

use super::context::{SelectorSite, selector_site};
use super::schema_hover::schema_value_candidates;
use crate::workspace::LiveProjectSnapshot;

mod presentation;
mod projection;

pub(super) fn completion(
    text: &str,
    position: Position,
    schema: &ProjectSchema,
    schema_authoring: bool,
    snapshot: &LiveProjectSnapshot,
    catalog: &UiCatalog,
) -> Option<CompletionResponse> {
    let line = super::line_prefix(text, position)?;
    if schema_authoring
        && let Some(items) =
            projection::schema_json_completion_items(text, position, line, schema, catalog)
    {
        return Some(items);
    }
    match completion_context(line) {
        CompletionContext::BlockReference => Some(items(block_completion_items(snapshot, catalog))),
        CompletionContext::Speaker => Some(items(speaker_completion_items(schema, catalog))),
        CompletionContext::MetadataKey => {
            Some(items(metadata_key_completion_items(schema, catalog)))
        }
        CompletionContext::MetadataValue { key, site } => {
            let line_index = usize::try_from(position.line).ok()?;
            let line_text = text.lines().nth(line_index)?;
            Some(items(metadata_value_completion_items(
                schema, text, line_text, line_index, &key, site, catalog,
            )))
        }
        CompletionContext::Condition => Some(items(presentation::condition_completion_items(
            schema, catalog,
        ))),
        CompletionContext::Effect => Some(items(presentation::effect_completion_items(
            schema, catalog,
        ))),
        CompletionContext::Reason => Some(CompletionResponse::Array(
            schema
                .availability_reasons
                .iter()
                .filter(|(_, definition)| definition.params.is_empty())
                .map(|(id, definition)| CompletionItem {
                    label: id.as_str().to_owned(),
                    kind: Some(CompletionItemKind::CONSTANT),
                    detail: Some(catalog.text(MsgId::LspCompletionAvailabilityReason)),
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
    MetadataValue { key: String, site: SelectorSite },
    Condition,
    Effect,
    Reason,
    None,
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

    let site = selector_site(line_prefix);
    if let Some(token) = current_token(line_prefix) {
        if token.starts_with("speaker=") && !matches!(site, Some(SelectorSite::Choice) | None) {
            return CompletionContext::Speaker;
        }
        if let Some((key, _)) = token.split_once('=')
            && !key.is_empty()
            && let Some(site) = site
        {
            return CompletionContext::MetadataValue {
                key: key.to_owned(),
                site,
            };
        }
    }

    if is_metadata_key_position(line_prefix, site) {
        return CompletionContext::MetadataKey;
    }

    CompletionContext::None
}

fn items(items: Vec<CompletionItem>) -> CompletionResponse {
    CompletionResponse::Array(items)
}

fn block_completion_items(
    snapshot: &LiveProjectSnapshot,
    catalog: &UiCatalog,
) -> Vec<CompletionItem> {
    super::block_names(snapshot)
        .into_iter()
        .map(|name| CompletionItem {
            label: name,
            kind: Some(CompletionItemKind::REFERENCE),
            detail: Some(catalog.text(MsgId::LspCompletionBlock)),
            ..CompletionItem::default()
        })
        .collect()
}

fn speaker_completion_items(schema: &ProjectSchema, catalog: &UiCatalog) -> Vec<CompletionItem> {
    schema
        .speakers
        .iter()
        .map(|(id, definition)| CompletionItem {
            label: id.clone(),
            kind: Some(CompletionItemKind::CONSTANT),
            detail: Some(catalog.text(MsgId::LspCompletionSpeaker)),
            documentation: definition
                .display_name
                .as_ref()
                .map(|display_name| Documentation::String(display_name.clone())),
            ..CompletionItem::default()
        })
        .collect()
}

fn metadata_key_completion_items(
    schema: &ProjectSchema,
    catalog: &UiCatalog,
) -> Vec<CompletionItem> {
    schema
        .metadata
        .iter()
        .map(|(key, definition)| CompletionItem {
            label: key.clone(),
            kind: Some(CompletionItemKind::FIELD),
            detail: Some(definition.domain.as_ref().map_or_else(
                || catalog.text(MsgId::LspCompletionMetadataKey),
                |domain| {
                    catalog.format_pairs(
                        MsgId::LspCompletionMetadataKeyWithDomain,
                        [("domain", domain.as_str())],
                    )
                },
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
    site: SelectorSite,
    catalog: &UiCatalog,
) -> Vec<CompletionItem> {
    let detail = schema.metadata.get(key).map_or_else(
        || catalog.text(MsgId::LspCompletionMetadataKey),
        |metadata| {
            metadata.domain.as_deref().map_or_else(
                || match metadata.type_ref {
                    SchemaTypeRef::Speaker => catalog.text(MsgId::LspCompletionSpeaker),
                    _ => catalog.text(MsgId::LspCompletionMetadataKey),
                },
                |domain| {
                    catalog.format_pairs(MsgId::LspCompletionMetadataDomain, [("domain", domain)])
                },
            )
        },
    );
    schema_value_candidates(schema, key, text, line, line_index, site)
        .into_iter()
        .map(|value| CompletionItem {
            label: value,
            kind: Some(CompletionItemKind::VALUE),
            detail: Some(detail.clone()),
            ..CompletionItem::default()
        })
        .collect()
}

fn current_token(line_prefix: &str) -> Option<&str> {
    line_prefix.split_whitespace().last()
}

fn is_metadata_key_position(line_prefix: &str, site: Option<SelectorSite>) -> bool {
    let Some(site) = site else {
        return false;
    };
    let Some(token) = current_token(line_prefix) else {
        return false;
    };
    if token.is_empty() || token.contains('=') {
        return false;
    }
    let field_count = line_prefix.split_whitespace().count();
    match site {
        SelectorSite::Block => field_count >= 3 && !("default".starts_with(token)),
        SelectorSite::Line | SelectorSite::Choice => field_count >= 3,
    }
}

fn effect_prefix_is_completing_function(line_prefix: &str) -> bool {
    let mut parts = line_prefix.split_whitespace();
    matches!(parts.next(), Some("!"))
        && parts.next().is_some()
        && parts.next().is_none_or(|function| !function.contains('('))
}
