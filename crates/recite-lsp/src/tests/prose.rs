use std::path::Path;
use std::time::Duration;

use lsp_types::{
    CodeActionContext, CodeActionOrCommand, CodeActionParams, DiagnosticSeverity,
    DocumentChangeOperation, DocumentChanges, NumberOrString, OneOf, PartialResultParams, Position,
    PublishDiagnosticsParams, Range, ResourceOp, TextDocumentIdentifier, TextEdit, Uri,
    WorkDoneProgressParams,
};
use tempfile::TempDir;

use super::support::{Harness, file_uri, harness_for_root, write_file};

const HARBOUR: &str = include_str!("../../../../fixtures/recite/valid/prose/harbour.recite");
const WORDS: &str = include_str!("../../../../fixtures/recite/valid/prose/words.txt");
const UNICODE: &str = ":: unicode_block\n> speaker=rhea bind=(gift:string=$gift) @unicode_line\n  Zoë 🙂 recieved the {gift} early.\n";

fn project(root: &Path, enabled: bool) {
    write_file(
        root,
        "recite.project.toml",
        &format!(
            "format_version = 1\n\n[prose]\nenabled = {enabled}\nlanguage = \"en-GB\"\ndictionaries = [\"words.txt\"]\n"
        ),
    );
    write_file(root, "words.txt", WORDS);
    write_file(root, "dialogue/harbour.recite", HARBOUR);
    write_file(root, "dialogue/unicode.recite", UNICODE);
}

fn open(harness: &Harness, uri: &Uri, text: &str) -> PublishDiagnosticsParams {
    harness.did_open(uri.clone(), 1, text);
    loop {
        let published = harness.recv_publish_diagnostics_within(Duration::from_secs(120));
        if &published.uri == uri {
            return published;
        }
    }
}

fn prose_codes(published: &PublishDiagnosticsParams) -> Vec<String> {
    published
        .diagnostics
        .iter()
        .filter_map(|diagnostic| match &diagnostic.code {
            Some(NumberOrString::String(code)) if code.starts_with("RECITE_PROSE") => {
                Some(code.clone())
            }
            _ => None,
        })
        .collect()
}

fn text_at(text: &str, range: Range) -> String {
    let line = text
        .lines()
        .nth(range.start.line as usize)
        .unwrap_or_default();
    let units = line.encode_utf16().collect::<Vec<_>>();
    String::from_utf16_lossy(&units[range.start.character as usize..range.end.character as usize])
}

fn actions_at(harness: &mut Harness, uri: &Uri, range: Range) -> Vec<lsp_types::CodeAction> {
    harness
        .code_action(CodeActionParams {
            text_document: TextDocumentIdentifier { uri: uri.clone() },
            range,
            context: CodeActionContext::default(),
            work_done_progress_params: WorkDoneProgressParams::default(),
            partial_result_params: PartialResultParams::default(),
        })
        .unwrap_or_default()
        .into_iter()
        .filter_map(|action| match action {
            CodeActionOrCommand::CodeAction(action) => Some(action),
            CodeActionOrCommand::Command(_) => None,
        })
        .collect()
}

fn single_edit(action: &lsp_types::CodeAction) -> (Uri, Option<i32>, TextEdit) {
    let Some(DocumentChanges::Edits(edits)) = action
        .edit
        .as_ref()
        .and_then(|edit| edit.document_changes.clone())
    else {
        panic!("expected text document edits: {action:?}");
    };
    let [edit] = edits.as_slice() else {
        panic!("expected one document edit: {edits:?}");
    };
    let [OneOf::Left(text_edit)] = edit.edits.as_slice() else {
        panic!("expected one text edit: {edit:?}");
    };
    (
        edit.text_document.uri.clone(),
        edit.text_document.version,
        text_edit.clone(),
    )
}

pub(super) fn prose_diagnostics_are_opt_in_per_project() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    project(temp.path(), false);
    let harness = harness_for_root(temp.path());
    let uri = file_uri(&temp.path().join("dialogue/harbour.recite"));
    let published = open(&harness, &uri, HARBOUR);
    assert!(prose_codes(&published).is_empty(), "{published:?}");
    harness.finish();
}

pub(super) fn prose_diagnostics_cover_prose_only() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    project(temp.path(), true);
    let harness = harness_for_root(temp.path());
    let uri = file_uri(&temp.path().join("dialogue/harbour.recite"));
    let published = open(&harness, &uri, HARBOUR);
    assert_eq!(
        prose_codes(&published),
        ["RECITE_PROSE001", "RECITE_PROSE002", "RECITE_PROSE001"]
    );
    let flagged = published
        .diagnostics
        .iter()
        .map(|diagnostic| {
            assert_eq!(diagnostic.severity, Some(DiagnosticSeverity::WARNING));
            text_at(HARBOUR, diagnostic.range)
        })
        .collect::<Vec<_>>();
    assert_eq!(flagged, ["recieved", "an", "befre"]);
    harness.finish();
}

pub(super) fn prose_quick_fixes_use_utf16_ranges_and_edit_prose_only() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    project(temp.path(), true);
    let mut harness = harness_for_root(temp.path());
    let uri = file_uri(&temp.path().join("dialogue/unicode.recite"));
    let published = open(&harness, &uri, UNICODE);
    let [diagnostic] = published.diagnostics.as_slice() else {
        panic!("expected one prose finding: {published:?}");
    };
    // `Zoë 🙂 ` is seven UTF-16 code units after the two-space indent.
    assert_eq!(
        diagnostic.range,
        Range::new(Position::new(2, 9), Position::new(2, 17))
    );

    let actions = actions_at(&mut harness, &uri, diagnostic.range);
    let titles = actions
        .iter()
        .map(|action| action.title.as_str())
        .collect::<Vec<_>>();
    assert_eq!(titles[0], "Replace with “received”");
    assert!(titles.contains(&"Add “recieved” to the project dictionary"));

    let (edit_uri, version, edit) = single_edit(&actions[0]);
    assert_eq!((edit_uri, version), (uri.clone(), Some(1)));
    assert_eq!(edit.range, diagnostic.range);
    let line = UNICODE.lines().nth(2).unwrap_or_default();
    let units = line.encode_utf16().collect::<Vec<_>>();
    let mut edited = units[..edit.range.start.character as usize].to_vec();
    edited.extend(edit.new_text.encode_utf16());
    edited.extend(&units[edit.range.end.character as usize..]);
    assert_eq!(
        String::from_utf16_lossy(&edited),
        "  Zoë 🙂 received the {gift} early."
    );

    let add = actions
        .iter()
        .find(|action| action.title.starts_with("Add"))
        .unwrap_or_else(|| panic!("add-word action"));
    let (dictionary, version, edit) = single_edit(add);
    assert_eq!(dictionary, file_uri(&temp.path().join("words.txt")));
    assert_eq!(version, None);
    let end_line = u32::try_from(WORDS.lines().count()).unwrap_or(u32::MAX);
    assert_eq!(
        edit.range,
        Range::new(Position::new(end_line, 0), Position::new(end_line, 0))
    );
    assert_eq!(edit.new_text, "recieved\n");
    harness.finish();
}

pub(super) fn add_word_creates_a_missing_dictionary() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    project(temp.path(), true);
    std::fs::remove_file(temp.path().join("words.txt"))
        .unwrap_or_else(|error| panic!("remove dictionary: {error}"));
    let mut harness = harness_for_root(temp.path());
    let uri = file_uri(&temp.path().join("dialogue/unicode.recite"));
    let published = open(&harness, &uri, UNICODE);
    let range = published.diagnostics[0].range;
    let add = actions_at(&mut harness, &uri, range)
        .into_iter()
        .find(|action| action.title.starts_with("Add"))
        .unwrap_or_else(|| panic!("add-word action"));
    let Some(DocumentChanges::Operations(operations)) =
        add.edit.and_then(|edit| edit.document_changes)
    else {
        panic!("expected resource operations");
    };
    assert!(matches!(
        &operations[0],
        DocumentChangeOperation::Op(ResourceOp::Create(create))
            if create.uri == file_uri(&temp.path().join("words.txt"))
    ));
    assert!(matches!(&operations[1], DocumentChangeOperation::Edit(edit)
        if matches!(edit.edits.as_slice(), [OneOf::Left(text)] if text.new_text == "recieved\n")));
    harness.finish();
}
