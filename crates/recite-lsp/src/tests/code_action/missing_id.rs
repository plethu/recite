use lsp_types::request::{CodeActionRequest, Request as LspRequest};
use lsp_types::{
    CodeActionKind, CodeActionOrCommand, CodeActionProviderCapability, DocumentChanges, OneOf,
    TextEdit,
};
use serde_json::json;
use tempfile::TempDir;

use super::support::{
    apply_edits, code_actions, fix_all, inserted_id, plain_text_edits, range, single_quick_fix,
    single_text_edit,
};
use crate::tests::support::{Harness, file_uri, harness_for_root, uri, write_file};

pub(super) fn initialize_advertises_missing_id_code_actions() {
    let (harness, result) = Harness::start_with_result(json!({
        "capabilities": {
            "general": {
                "positionEncodings": ["utf-16"]
            }
        }
    }));

    let Some(CodeActionProviderCapability::Options(options)) =
        result.capabilities.code_action_provider
    else {
        panic!("expected code action options");
    };
    assert_eq!(
        options.code_action_kinds,
        Some(vec![
            CodeActionKind::QUICKFIX,
            CodeActionKind::SOURCE_FIX_ALL,
            CodeActionKind::REFACTOR,
        ])
    );

    harness.finish();
}
pub(super) fn quick_fix_inserts_marker_only_line_and_choice_ids() {
    let mut harness = Harness::start();
    let source_uri = uri("file:///workspace/dialogue/code-action.recite");
    let source = concat!(
        ":: start default\n",
        ">\n",
        "  Hello.\n",
        "?\n",
        "  Stay.\n"
    );
    harness.did_open(source_uri.clone(), 7, source);
    let _ = harness.recv_publish_diagnostics();

    let line_action = single_quick_fix(&mut harness, source_uri.clone(), range(1, 0, 1, 1));
    let line_edit = single_text_edit(&line_action);
    assert_eq!(line_action.text_document.version, Some(7));
    assert_eq!(line_edit.range, range(1, 1, 1, 1));
    assert_generated_insert(&line_edit.new_text, "line");

    let choice_action = single_quick_fix(&mut harness, source_uri, range(3, 0, 3, 1));
    let choice_edit = single_text_edit(&choice_action);
    assert_eq!(choice_edit.range, range(3, 1, 3, 1));
    assert_generated_insert(&choice_edit.new_text, "choice");

    harness.finish();
}

pub(super) fn quick_fix_preserves_spacing_for_metadata_and_clauses_first_headers() {
    let mut harness = Harness::start();
    let source_uri = uri("file:///workspace/dialogue/code-action-spacing.recite");
    let source = concat!(
        ":: start default\n",
        "> speaker=rhea\n",
        "  Hello.\n",
        ">speaker=rhea\n",
        "  Tight.\n",
        "? requires=(trusts(player))\n",
        "  Ask.\n",
        "?requires=(trusts(player))\n",
        "  Tight ask.\n",
    );
    harness.did_open(source_uri.clone(), 1, source);
    let _ = harness.recv_publish_diagnostics();

    let spaced_line = single_quick_fix(&mut harness, source_uri.clone(), range(1, 0, 1, 1));
    let spaced_line_edit = single_text_edit(&spaced_line);
    assert_eq!(spaced_line_edit.range, range(1, 1, 1, 1));
    assert!(spaced_line_edit.new_text.starts_with(" line@"));
    assert!(!spaced_line_edit.new_text.ends_with(' '));

    let tight_line = single_quick_fix(&mut harness, source_uri.clone(), range(3, 0, 3, 1));
    let tight_line_edit = single_text_edit(&tight_line);
    assert_eq!(tight_line_edit.range, range(3, 1, 3, 1));
    assert!(tight_line_edit.new_text.starts_with(" line_2@"));
    assert!(tight_line_edit.new_text.ends_with(' '));

    let spaced_choice = single_quick_fix(&mut harness, source_uri.clone(), range(5, 0, 5, 1));
    let spaced_choice_edit = single_text_edit(&spaced_choice);
    assert_eq!(spaced_choice_edit.range, range(5, 1, 5, 1));
    assert!(spaced_choice_edit.new_text.starts_with(" choice@"));
    assert!(!spaced_choice_edit.new_text.ends_with(' '));

    let tight_choice = single_quick_fix(&mut harness, source_uri, range(7, 0, 7, 1));
    let tight_choice_edit = single_text_edit(&tight_choice);
    assert_eq!(tight_choice_edit.range, range(7, 1, 7, 1));
    assert!(tight_choice_edit.new_text.starts_with(" choice_2@"));
    assert!(tight_choice_edit.new_text.ends_with(' '));

    harness.finish();
}

pub(super) fn quick_fix_freezes_draft_and_plain_label_headers() {
    let mut harness = Harness::start();
    let source_uri = uri("file:///workspace/dialogue/code-action-draft.recite");
    let source = concat!(
        ":: start default\n",
        "> greeting@\n",
        "  Hello.\n",
        "? answer_anywhere\n",
        "  Anywhere.\n",
    );
    harness.did_open(source_uri.clone(), 3, source);
    let _ = harness.recv_publish_diagnostics();

    let draft_line = single_quick_fix(&mut harness, source_uri.clone(), range(1, 0, 1, 10));
    let draft_line_edit = single_text_edit(&draft_line);
    assert_eq!(draft_line_edit.range, range(1, 11, 1, 11));
    assert_anchor_only_insert(&draft_line_edit.new_text);
    let applied = apply_edits(source, &[draft_line_edit]);
    assert!(applied.contains("> greeting@"));
    assert!(!applied.contains("> greeting@@"));

    let plain_choice = single_quick_fix(&mut harness, source_uri, range(3, 0, 3, 17));
    let plain_choice_edit = single_text_edit(&plain_choice);
    assert_eq!(plain_choice_edit.range, range(3, 17, 3, 17));
    assert_at_anchor_insert(&plain_choice_edit.new_text);
    let applied = apply_edits(source, &[plain_choice_edit]);
    assert!(applied.contains("? answer_anywhere@"));
    assert!(!applied.contains("? answer_anywhere @"));

    harness.finish();
}

pub(super) fn source_fix_all_orders_deterministic_multi_edits_and_preserves_existing_ids() {
    let mut harness = Harness::start();
    let source_uri = uri("file:///workspace/dialogue/code-action-fix-all.recite");
    let source = concat!(
        ":: start default\n",
        "> existing_line@af869b29ee4045f73952\n",
        "  Existing.\n",
        ">\n",
        "  Missing line.\n",
        "? existing_choice@e2b17e31e46100680de3\n",
        "  Existing choice.\n",
        "?\n",
        "  Missing choice.\n",
    );
    harness.did_open(source_uri.clone(), 2, source);
    let _ = harness.recv_publish_diagnostics();

    let edit = fix_all(&mut harness, source_uri, range(0, 0, 0, 0));
    assert_eq!(edit.text_document.version, Some(2));
    let edits = plain_text_edits(&edit);
    assert_eq!(edits.len(), 2);
    assert_eq!(edits[0].range, range(3, 1, 3, 1));
    assert_eq!(edits[1].range, range(7, 1, 7, 1));
    let applied = apply_edits(source, &edits);
    assert!(applied.contains("> existing_line@af869b29ee4045f73952\n"));
    assert!(applied.contains("? existing_choice@e2b17e31e46100680de3\n"));
    assert!(applied.contains("> line@"));
    assert!(applied.contains("? choice@"));

    harness.finish();
}

pub(super) fn source_fix_all_scopes_edits_and_guards_sibling_documents() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "a.recite", ":: a\n>\n  Missing.\n");
    write_file(
        temp.path(),
        "b.recite",
        ":: b\n> existing@0123456789abcdef0123\n  Existing.\n",
    );
    let a_uri = file_uri(&temp.path().join("a.recite"));
    let b_uri = file_uri(&temp.path().join("b.recite"));
    let mut harness = harness_for_root(temp.path());
    harness.did_open(
        b_uri.clone(),
        9,
        ":: b\n> existing@0123456789abcdef0123\n  Existing.\n",
    );
    let _ = harness.recv_publish_diagnostics();
    let actions = code_actions(
        &mut harness,
        a_uri.clone(),
        range(0, 0, 3, 10),
        Some(vec![CodeActionKind::SOURCE_FIX_ALL]),
    );
    let Some(CodeActionOrCommand::CodeAction(action)) = actions.into_iter().next() else {
        panic!("expected source.fixAll action");
    };
    let Some(DocumentChanges::Edits(changes)) = action.edit.and_then(|edit| edit.document_changes)
    else {
        panic!("expected guarded document changes");
    };
    assert_eq!(changes.len(), 2);
    let a_change = changes
        .iter()
        .find(|change| change.text_document.uri == a_uri);
    let b_change = changes
        .iter()
        .find(|change| change.text_document.uri == b_uri);
    assert_eq!(
        a_change.and_then(|change| change.text_document.version),
        None
    );
    assert_eq!(
        b_change.and_then(|change| change.text_document.version),
        Some(9)
    );
    assert_eq!(a_change.map(|change| change.edits.len()), Some(1));
    assert_eq!(b_change.map(|change| change.edits.len()), Some(0));
    assert!(matches!(
        a_change.and_then(|change| change.edits.first()),
        Some(OneOf::Left(_))
    ));
    harness.finish();
}

pub(super) fn quick_fix_full_document_range_uses_bounded_candidates() {
    let mut harness = Harness::start();
    let source_uri = uri("file:///workspace/dialogue/code-action-large-range.recite");
    let prose = "ordinary prose ".repeat(4_000);
    let source = format!(":: start default\n>\n  {prose}\n");
    harness.did_open(source_uri.clone(), 6, &source);
    let _ = harness.recv_publish_diagnostics();

    let action = single_quick_fix(
        &mut harness,
        source_uri,
        range(0, 0, 2, (prose.len() + 2) as u32),
    );
    assert_generated_insert(&single_text_edit(&action).new_text, "line");
    harness.finish();
}

pub(super) fn generated_ids_are_deterministic_and_avoid_line_choice_namespace_collisions() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let source_path = temp.path().join("start.recite");
    let source_uri = file_uri(&source_path);
    let source = concat!(":: start default\n", ">\n", "  Missing.\n");
    write_file(temp.path(), "start.recite", source);

    let first_id = {
        let mut harness = harness_for_root(temp.path());
        let edit = fix_all(&mut harness, source_uri.clone(), range(0, 0, 2, 10));
        let id = inserted_id(&plain_text_edits(&edit)[0]);
        harness.finish();
        id
    };

    let repeated_id = {
        let mut harness = harness_for_root(temp.path());
        let edit = fix_all(&mut harness, source_uri.clone(), range(0, 0, 2, 10));
        let id = inserted_id(&plain_text_edits(&edit)[0]);
        harness.finish();
        id
    };
    assert_eq!(first_id, repeated_id);

    write_file(
        temp.path(),
        "occupied.recite",
        &format!(":: occupied\n? {first_id}\n  Existing choice.\n"),
    );
    let mut harness = harness_for_root(temp.path());
    let edit = fix_all(&mut harness, source_uri, range(0, 0, 2, 10));
    let collision_safe_id = inserted_id(&plain_text_edits(&edit)[0]);
    assert_ne!(collision_safe_id, first_id);

    harness.finish();
}

pub(super) fn code_actions_use_utf16_crlf_and_indented_ranges() {
    let mut harness = Harness::start();
    let source_uri = uri("file:///workspace/dialogue/code-action-utf16.recite");
    let source = ":: start default\r\n  > speaker=é\r\n    Hello.\r\n";
    harness.did_open(source_uri.clone(), 4, source);
    let _ = harness.recv_publish_diagnostics();

    let edit = single_quick_fix(&mut harness, source_uri, range(1, 2, 1, 3));
    let text_edit = single_text_edit(&edit);
    assert_eq!(text_edit.range, range(1, 3, 1, 3));
    assert_generated_insert(&text_edit.new_text, "line");

    harness.finish();
}

pub(super) fn existing_and_draft_stem_ids_do_not_receive_missing_id_actions() {
    let mut harness = Harness::start();
    let source_uri = uri("file:///workspace/dialogue/code-action-existing.recite");
    let source = concat!(
        ":: start default\n",
        "> hazel_rhea.small_talk@be29420d780048facdc3\n",
        "  Existing draft-stem-shaped ID remains out of scope for #33.\n",
        "? existing_choice@f926a9508c84a21da386\n",
        "  Existing choice.\n",
    );
    harness.did_open(source_uri.clone(), 1, source);
    let _ = harness.recv_publish_diagnostics();

    let actions = code_actions(&mut harness, source_uri.clone(), range(1, 0, 1, 1), None);
    assert!(actions.is_empty());
    let actions = code_actions(&mut harness, source_uri, range(3, 0, 3, 1), None);
    assert!(actions.is_empty());

    harness.finish();
}

pub(super) fn malformed_code_action_params_return_invalid_params() {
    let mut harness = Harness::start();

    let response = harness.raw_request_response(CodeActionRequest::METHOD, json!({"bad": true}));
    assert_eq!(
        response.error.expect("code action error").code,
        lsp_server::ErrorCode::InvalidParams as i32
    );

    harness.finish();
}

fn assert_generated_insert(insert: &str, label: &str) {
    let id = inserted_id(&TextEdit {
        range: range(0, 0, 0, 0),
        new_text: insert.to_owned(),
    });
    let Some(anchor) = id.strip_prefix(&format!("{label}@")) else {
        panic!("generated ID `{id}` did not start with `{label}@`");
    };
    assert_eq!(anchor.len(), 20);
    assert!(
        anchor
            .chars()
            .all(|character| character.is_ascii_hexdigit() && !character.is_ascii_uppercase()),
        "anchor must be lowercase hex: {anchor}"
    );
}

fn assert_anchor_only_insert(insert: &str) {
    assert_eq!(insert.len(), 20);
    assert!(
        insert
            .chars()
            .all(|character| character.is_ascii_hexdigit() && !character.is_ascii_uppercase()),
        "anchor must be lowercase hex: {insert}"
    );
}

fn assert_at_anchor_insert(insert: &str) {
    let Some(anchor) = insert.strip_prefix('@') else {
        panic!("insert `{insert}` did not start with `@`");
    };
    assert_anchor_only_insert(anchor);
}
