mod block_stub;
mod missing_id;
mod schema_entry;

use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, CodeActionResponse,
    DocumentChanges, OneOf, OptionalVersionedTextDocumentIdentifier, Position, Range,
    TextDocumentEdit, TextEdit, Uri, WorkspaceEdit,
};
use recite_compiler::AuthoringSnapshot;
use recite_core::SchemaSource;
use recite_ui::{MsgId, UiCatalog};

use crate::edit_projection::EditDocument;
use crate::summary::FileSummary;

pub(crate) struct CodeActionDocument<'a> {
    pub(crate) source: EditDocument<'a>,
    pub(crate) summary: &'a FileSummary,
}

pub(crate) struct SchemaCodeActionDocument {
    pub(crate) uri: Uri,
    pub(crate) text: String,
    pub(crate) summary: recite_compiler::SchemaSummary,
    pub(crate) source: SchemaSource,
    pub(crate) version: Option<i32>,
}

pub(crate) fn code_action(
    params: &CodeActionParams,
    snapshot: &AuthoringSnapshot,
    documents: &[CodeActionDocument<'_>],
    schema: Option<SchemaCodeActionDocument>,
    catalog: &UiCatalog,
) -> Option<CodeActionResponse> {
    let document = documents
        .iter()
        .find(|document| *document.source.uri == params.text_document.uri)?;
    let edit_documents = documents
        .iter()
        .map(|document| document.source)
        .collect::<Vec<_>>();

    let mut actions = Vec::new();
    let include_quick_fix = includes_kind(params, &CodeActionKind::QUICKFIX);
    let include_fix_all = includes_kind(params, &CodeActionKind::SOURCE_FIX_ALL);

    let quick_fix_edit = if include_quick_fix {
        missing_id::edit(document, snapshot, &edit_documents, params.range)
    } else {
        None
    };
    if let Some(edit) = quick_fix_edit {
        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
            title: catalog.text(MsgId::LspCodeActionInsertMissingId),
            kind: Some(CodeActionKind::QUICKFIX),
            diagnostics: Some(params.context.diagnostics.clone()),
            edit: Some(edit),
            ..CodeAction::default()
        }));
    }
    if include_quick_fix {
        actions.extend(block_stub::actions(
            params,
            document,
            snapshot,
            &edit_documents,
            catalog,
        ));
        if let Some(schema) = schema {
            actions.extend(schema_entry::actions(
                params, document, documents, &schema, catalog,
            ));
        }
    }

    let fix_all_edit = if include_fix_all {
        missing_id::fix_all(document, snapshot, &edit_documents)
    } else {
        None
    };
    if let Some(edit) = fix_all_edit {
        actions.push(CodeActionOrCommand::CodeAction(CodeAction {
            title: catalog.text(MsgId::LspCodeActionInsertAllMissingIds),
            kind: Some(CodeActionKind::SOURCE_FIX_ALL),
            edit: Some(edit),
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

fn full_document_range(text: &str) -> Range {
    let lines = text.split('\n').collect::<Vec<_>>();
    let (line, character) = if text.ends_with('\n') {
        (lines.len().saturating_sub(1), 0)
    } else {
        let last = lines
            .last()
            .copied()
            .unwrap_or_default()
            .trim_end_matches('\r');
        (lines.len().saturating_sub(1), last.encode_utf16().count())
    };
    Range {
        start: Position::new(0, 0),
        end: Position::new(line as u32, character as u32),
    }
}

fn ranges_intersect(left: Range, right: Range) -> bool {
    position_le(left.start, right.end) && position_le(right.start, left.end)
}

fn position_le(left: Position, right: Position) -> bool {
    left.line < right.line || (left.line == right.line && left.character <= right.character)
}
