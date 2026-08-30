use lsp_types::{
    CodeActionContext, CodeActionKind, CodeActionOrCommand, CodeActionParams, DocumentChanges,
    OneOf, Position, Range, TextDocumentEdit, TextDocumentIdentifier, TextEdit, Uri,
};
use serde_json::{Value, json};

use crate::tests::support::{Harness, file_uri};

pub(super) fn single_quick_fix(harness: &mut Harness, uri: Uri, range: Range) -> TextDocumentEdit {
    single_quick_fix_matching(harness, uri, range, |_| true)
}

pub(super) fn single_quick_fix_with_title(
    harness: &mut Harness,
    uri: Uri,
    range: Range,
    title: &str,
) -> TextDocumentEdit {
    single_quick_fix_matching(harness, uri, range, |action| action.title == title)
}

fn single_quick_fix_matching(
    harness: &mut Harness,
    uri: Uri,
    range: Range,
    predicate: impl Fn(&lsp_types::CodeAction) -> bool,
) -> TextDocumentEdit {
    let actions = code_actions(harness, uri, range, Some(vec![CodeActionKind::QUICKFIX]));
    let edits = actions
        .into_iter()
        .filter_map(|action| match action {
            CodeActionOrCommand::CodeAction(action)
                if action.kind == Some(CodeActionKind::QUICKFIX) && predicate(&action) =>
            {
                action.edit
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(edits.len(), 1, "expected one quick-fix edit");
    single_document_edit(edits.into_iter().next().expect("quick-fix edit"))
}

pub(super) fn assert_no_action_title(actions: &[CodeActionOrCommand], title: &str) {
    assert!(
        actions.iter().all(|action| match action {
            CodeActionOrCommand::CodeAction(action) => action.title != title,
            CodeActionOrCommand::Command(_) => true,
        }),
        "unexpected action `{title}`"
    );
}

pub(super) fn fix_all(harness: &mut Harness, uri: Uri, range: Range) -> TextDocumentEdit {
    let actions = code_actions(
        harness,
        uri,
        range,
        Some(vec![CodeActionKind::SOURCE_FIX_ALL]),
    );
    let edits = actions
        .into_iter()
        .filter_map(|action| match action {
            CodeActionOrCommand::CodeAction(action)
                if action.kind == Some(CodeActionKind::SOURCE_FIX_ALL) =>
            {
                action.edit
            }
            _ => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(edits.len(), 1, "expected one source.fixAll edit");
    single_document_edit(edits.into_iter().next().expect("source.fixAll edit"))
}

pub(super) fn code_actions(
    harness: &mut Harness,
    uri: Uri,
    range: Range,
    only: Option<Vec<CodeActionKind>>,
) -> Vec<CodeActionOrCommand> {
    harness
        .code_action(CodeActionParams {
            text_document: TextDocumentIdentifier { uri },
            range,
            context: CodeActionContext {
                diagnostics: Vec::new(),
                only,
                trigger_kind: None,
            },
            work_done_progress_params: Default::default(),
            partial_result_params: Default::default(),
        })
        .expect("code action response")
}

fn single_document_edit(edit: lsp_types::WorkspaceEdit) -> TextDocumentEdit {
    let Some(DocumentChanges::Edits(changes)) = edit.document_changes else {
        panic!("expected document changes");
    };
    let edited = changes
        .into_iter()
        .filter(|change| !change.edits.is_empty())
        .collect::<Vec<_>>();
    assert_eq!(edited.len(), 1);
    edited.into_iter().next().expect("edited document")
}

pub(super) fn single_text_edit(edit: &TextDocumentEdit) -> TextEdit {
    let edits = plain_text_edits(edit);
    assert_eq!(edits.len(), 1);
    edits[0].clone()
}

pub(super) fn plain_text_edits(edit: &TextDocumentEdit) -> Vec<TextEdit> {
    edit.edits
        .iter()
        .map(|edit| match edit {
            OneOf::Left(edit) => edit.clone(),
            OneOf::Right(_) => panic!("expected plain text edit"),
        })
        .collect()
}

pub(super) fn inserted_id(edit: &TextEdit) -> String {
    edit.new_text
        .split_whitespace()
        .next()
        .expect("inserted ID")
        .to_owned()
}

pub(super) fn apply_edits(source: &str, edits: &[TextEdit]) -> String {
    let mut output = source.to_owned();
    for edit in edits.iter().rev() {
        let start = byte_offset_for_position(&output, edit.range.start);
        let end = byte_offset_for_position(&output, edit.range.end);
        output.replace_range(start..end, &edit.new_text);
    }
    output
}

fn byte_offset_for_position(text: &str, position: Position) -> usize {
    let mut offset = 0;
    for (line_index, line) in text.split_inclusive('\n').enumerate() {
        if line_index == position.line as usize {
            let line_without_newline = line.strip_suffix('\n').unwrap_or(line);
            return offset + byte_offset_for_utf16(line_without_newline, position.character);
        }
        offset += line.len();
    }
    text.len()
}

fn byte_offset_for_utf16(line: &str, character: u32) -> usize {
    let mut utf16 = 0_u32;
    for (byte_index, value) in line.char_indices() {
        if utf16 == character {
            return byte_index;
        }
        utf16 = utf16.saturating_add(value.len_utf16() as u32);
        if utf16 > character {
            return byte_index;
        }
    }
    line.len()
}

pub(super) fn range(
    start_line: u32,
    start_character: u32,
    end_line: u32,
    end_character: u32,
) -> Range {
    Range {
        start: Position::new(start_line, start_character),
        end: Position::new(end_line, end_character),
    }
}

pub(super) fn harness_for_root_with_schema(root: &std::path::Path) -> Harness {
    harness_for_root_with_schema_value(root, root.join("schema.json").display().to_string())
}

pub(super) fn harness_for_root_with_schema_value(
    root: &std::path::Path,
    schema: impl Into<Value>,
) -> Harness {
    let root_uri = file_uri(root);
    Harness::start_with_result(json!({
        "capabilities": {
            "general": {
                "positionEncodings": ["utf-16"]
            }
        },
        "rootUri": root_uri.as_str(),
        "initializationOptions": {
            "schema": schema.into()
        }
    }))
    .0
}

pub(super) fn schema_manifest(conditions: &str, effects: &str) -> String {
    format!(
        "{{\n  \"schema_version\": 1,\n  \"conditions\": {{{conditions}}},\n  \"effects\": {{{effects}}}\n}}\n"
    )
}
