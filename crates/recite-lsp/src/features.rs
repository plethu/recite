use std::collections::BTreeSet;

use lsp_types::{CodeActionParams, CodeActionResponse, CompletionResponse, Hover, Position};
use recite_compiler::AuthoringSnapshot;
use recite_core::{ConditionReturnType, DocumentKey, EffectMode, ProjectSchema, SchemaTypeRef};
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
    schema: Option<&ProjectSchema>,
    schema_authoring: bool,
    catalog: &UiCatalog,
) -> Option<CompletionResponse> {
    completion::completion(
        text,
        position,
        key,
        snapshot,
        schema,
        schema_authoring,
        catalog,
    )
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
    schema: Option<&ProjectSchema>,
    catalog: &UiCatalog,
) -> Option<Hover> {
    hover::hover(text, position, key, snapshot, schema, catalog)
}

pub(super) fn line_prefix(text: &str, position: Position) -> Option<&str> {
    let line = text.lines().nth(usize::try_from(position.line).ok()?)?;
    let end = byte_index_for_utf16_character(line, position.character)?;
    line.get(..end)
}

pub(super) use hover::byte_index_for_utf16_character;

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
