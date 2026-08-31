mod block_stub;
mod missing_id;
mod schema_entry;

use lsp_types::{
    CodeAction, CodeActionKind, CodeActionOrCommand, CodeActionParams, CodeActionResponse,
    DocumentChanges, OneOf, OptionalVersionedTextDocumentIdentifier, Position, Range,
    TextDocumentEdit, TextEdit, Uri, WorkspaceEdit,
};
use recite_compiler::AuthoringSnapshot;
use recite_core::{SchemaSource, SchemaSourceEditPlan};
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
    pub(crate) version: i32,
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

pub(crate) fn schema_workspace_edit(
    schema: &SchemaCodeActionDocument,
    plan: &SchemaSourceEditPlan,
    documents: &[CodeActionDocument<'_>],
) -> Option<WorkspaceEdit> {
    let mut source = schema.source.clone();
    plan.apply(&mut source).ok()?;
    if source.source_text() != plan.replacement_text()
        || schema.source.source_text() != schema.text
        || schema.source.source_text_fingerprint() != *plan.expected_text_fingerprint()
        || schema.source.source_fingerprint() != plan.expected_source_fingerprint()
    {
        return None;
    }

    let mut changes = vec![TextDocumentEdit {
        text_document: OptionalVersionedTextDocumentIdentifier {
            uri: schema.uri.clone(),
            version: Some(schema.version),
        },
        edits: vec![OneOf::Left(TextEdit {
            range: full_document_range(&schema.text),
            new_text: plan.replacement_text().to_owned(),
        })],
    }];
    changes.extend(
        documents
            .iter()
            .filter(|document| document.source.layer == recite_compiler::DocumentLayer::Open)
            .map(|document| TextDocumentEdit {
                text_document: OptionalVersionedTextDocumentIdentifier {
                    uri: document.source.uri.clone(),
                    version: document
                        .source
                        .version
                        .and_then(|version| i32::try_from(version.as_i64()).ok()),
                },
                edits: Vec::new(),
            }),
    );
    changes.sort_by(|left, right| {
        left.text_document
            .uri
            .as_str()
            .cmp(right.text_document.uri.as_str())
    });
    if changes
        .windows(2)
        .any(|pair| pair[0].text_document.uri == pair[1].text_document.uri)
    {
        return None;
    }
    Some(WorkspaceEdit {
        changes: None,
        document_changes: Some(DocumentChanges::Edits(changes)),
        change_annotations: None,
    })
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
