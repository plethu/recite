use std::collections::BTreeSet;

use lsp_types::{CodeActionParams, CodeActionResponse, CompletionResponse, Hover, Position};
use recite_compiler::AuthoringSnapshot;
use recite_compiler::SchemaSummary;
use recite_core::{ConditionReturnType, DocumentKey, EffectMode, SchemaTypeRef};
use recite_ui::UiCatalog;

mod code_action;
mod completion;
mod hover;
mod navigation;
pub(crate) mod schema_hover;

pub(crate) fn completion(
    text: &str,
    position: Position,
    key: Option<&DocumentKey>,
    snapshot: &AuthoringSnapshot,
    schema: Option<&SchemaSummary>,
    catalog: &UiCatalog,
) -> Option<CompletionResponse> {
    completion::completion(text, position, key, snapshot, schema, catalog)
}

pub(crate) use code_action::{CodeActionDocument, SchemaCodeActionDocument};

pub(crate) fn code_action(
    params: &CodeActionParams,
    snapshot: &AuthoringSnapshot,
    documents: &[CodeActionDocument<'_>],
    schema: Option<SchemaCodeActionDocument>,
    catalog: &UiCatalog,
) -> Option<CodeActionResponse> {
    code_action::code_action(params, snapshot, documents, schema, catalog)
}

pub(crate) use navigation::{NavigationDocument, definition, prepare_rename, references, rename};

pub(crate) fn hover(
    text: &str,
    position: Position,
    key: &DocumentKey,
    snapshot: &AuthoringSnapshot,
    schema: Option<&SchemaSummary>,
    catalog: &UiCatalog,
) -> Option<Hover> {
    hover::hover(text, position, key, snapshot, schema, catalog)
}

pub(super) fn condition_detail(return_type: &ConditionReturnType) -> String {
    match return_type {
        ConditionReturnType::Bool => "bool".to_owned(),
        ConditionReturnType::Enum(name) => format!("enum:{name}"),
    }
}

pub(super) fn effect_detail(modes: &BTreeSet<EffectMode>) -> String {
    modes
        .iter()
        .map(|mode| match mode {
            EffectMode::Immediate => "immediate",
            EffectMode::Deferred => "deferred",
            EffectMode::Blocking => "blocking",
        })
        .collect::<Vec<_>>()
        .join(", ")
}

pub(super) fn schema_type_detail(type_ref: &SchemaTypeRef) -> String {
    match type_ref {
        SchemaTypeRef::String => "string".to_owned(),
        SchemaTypeRef::Symbol => "symbol".to_owned(),
        SchemaTypeRef::Int => "int".to_owned(),
        SchemaTypeRef::Float => "float".to_owned(),
        SchemaTypeRef::Bool => "bool".to_owned(),
        SchemaTypeRef::Speaker => "speaker".to_owned(),
        SchemaTypeRef::Enum(name) => format!("enum:{name}"),
        SchemaTypeRef::Registry(name) => format!("registry:{name}"),
        SchemaTypeRef::Array(inner) => format!("array:{}", schema_type_detail(inner)),
    }
}
