mod block_stub;
mod json_edit;
mod missing_id;
mod schema_entry;

use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, CodeActionResponse,
    DocumentChanges, OneOf, OptionalVersionedTextDocumentIdentifier, Position, Range,
    TextDocumentEdit, TextEdit, Uri, WorkspaceEdit,
};
use recite_ui::{MsgId, UiCatalog};

use crate::summary::{FileSummary, SchemaSummary};

pub(crate) struct CodeActionDocument<'a> {
    pub(crate) uri: &'a Uri,
    pub(crate) text: &'a str,
    pub(crate) summary: &'a FileSummary,
}

pub(crate) struct SchemaCodeActionDocument<'a> {
    pub(crate) uri: &'a Uri,
    pub(crate) text: &'a str,
    pub(crate) summary: &'a SchemaSummary,
    pub(crate) version: Option<i32>,
}

pub(crate) fn code_action(
    params: &CodeActionParams,
    documents: &[CodeActionDocument<'_>],
    schema: Option<SchemaCodeActionDocument<'_>>,
    catalog: &UiCatalog,
) -> Option<CodeActionResponse> {
    let document = documents
        .iter()
        .find(|document| *document.uri == params.text_document.uri)?;

    let mut actions = Vec::new();
    let include_quick_fix = includes_kind(params, &CodeActionKind::QUICKFIX);
    let include_fix_all = includes_kind(params, &CodeActionKind::SOURCE_FIX_ALL);

    let quick_fix_edits = if include_quick_fix {
        missing_id::edits(document, documents, Some(params.range))
    } else {
        Vec::new()
    };
    if quick_fix_edits.len() == 1 {
        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
            title: catalog.text(MsgId::LspCodeActionInsertMissingId),
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: Some(params.context.diagnostics.clone()),
            edit: Some(workspace_edit(
                document.uri.clone(),
                document.summary.version,
                quick_fix_edits,
            )),
            ..CodeAction::default()
        }));
    }
    if include_quick_fix {
        actions.extend(block_stub::actions(params, document, documents, catalog));
        if let Some(schema) = schema {
            actions.extend(schema_entry::actions(
                params, document, documents, schema, catalog,
            ));
        }
    }

    let fix_all_edits = if include_fix_all {
        missing_id::edits(document, documents, None)
    } else {
        Vec::new()
    };
    if !fix_all_edits.is_empty() {
        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
            title: catalog.text(MsgId::LspCodeActionInsertAllMissingIds),
            kind: Some(CodeActionKind::SOURCE_FIX_ALL),
            edit: Some(workspace_edit(
                document.uri.clone(),
                document.summary.version,
                fix_all_edits,
            )),
            ..CodeAction::default()
        }));
    }

    Some(actions)
}

fn includes_kind(params: &CodeActionParams, kind: &CodeActionKind) -> bool {
    params
        .context
        .only
        .as_ref()
        .is_none_or(|kinds| kinds.iter().any(|candidate| candidate == kind))
}

fn workspace_edit(uri: Uri, version: Option<i32>, edits: Vec<TextEdit>) -> WorkspaceEdit {
    WorkspaceEdit {
        changes: None,
        document_changes: Some(DocumentChanges::Edits(vec![TextDocumentEdit {
            text_document: OptionalVersionedTextDocumentIdentifier { uri, version },
            edits: edits.into_iter().map(OneOf::Left).collect(),
        }])),
        change_annotations: None,
    }
}

fn ranges_intersect(left: Range, right: Range) -> bool {
    position_le(left.start, right.end) && position_le(right.start, left.end)
}

fn position_le(left: Position, right: Position) -> bool {
    left.line < right.line || (left.line == right.line && left.character <= right.character)
}

fn end_position(text: &str) -> Position {
    json_edit::byte_position(text, text.len())
}

fn newline_for(text: &str) -> &'static str {
    if text.contains("\r\n") { "\r\n" } else { "\n" }
}
