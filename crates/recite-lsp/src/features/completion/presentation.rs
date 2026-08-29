use lsp_types::{CompletionItem, CompletionItemKind, Documentation};
use recite_core::ProjectSchema;
use recite_ui::{MsgId, UiCatalog};

pub(super) fn condition_completion_items(
    schema: &ProjectSchema,
    catalog: &UiCatalog,
) -> Vec<CompletionItem> {
    schema
        .conditions
        .iter()
        .map(|(name, definition)| CompletionItem {
            label: name.clone(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some(catalog.format_pairs(
                MsgId::LspCompletionCondition,
                [(
                    "returns",
                    super::super::condition_detail(&definition.returns),
                )],
            )),
            documentation: Some(Documentation::String(
                catalog.text(MsgId::LspCompletionConditionDocumentation),
            )),
            ..CompletionItem::default()
        })
        .collect()
}

pub(super) fn effect_completion_items(
    schema: &ProjectSchema,
    catalog: &UiCatalog,
) -> Vec<CompletionItem> {
    schema
        .effects
        .iter()
        .map(|(name, definition)| CompletionItem {
            label: name.clone(),
            kind: Some(CompletionItemKind::FUNCTION),
            detail: Some(catalog.format_pairs(
                MsgId::LspCompletionEffect,
                [("modes", super::super::effect_detail(&definition.modes))],
            )),
            documentation: Some(Documentation::String(
                catalog.text(MsgId::LspCompletionEffectDocumentation),
            )),
            ..CompletionItem::default()
        })
        .collect()
}
