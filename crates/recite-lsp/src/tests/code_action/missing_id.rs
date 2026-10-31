use lsp_types::request::{CodeActionRequest, Request as LspRequest};
use lsp_types::{
    CodeActionKind, CodeActionOrCommand, CodeActionProviderCapability, DocumentChanges, OneOf,
};
use recite_core::{GENERATED_STABLE_ID_LEN, is_valid_stable_id};
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
    assert_generated_insert(&line_edit.new_text);

    let choice_action = single_quick_fix(&mut harness, source_uri, range(3, 0, 3, 1));
    let choice_edit = single_text_edit(&choice_action);
    assert_eq!(choice_edit.range, range(3, 1, 3, 1));
    assert_generated_insert(&choice_edit.new_text);

    harness.finish();
}

pub(super) fn quick_fix_appends_ids_after_metadata_and_clauses() {
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

    for (line, end) in [(1, 14), (3, 13), (5, 27), (7, 26)] {
        let action = single_quick_fix(&mut harness, source_uri.clone(), range(line, 0, line, 1));
        let edit = single_text_edit(&action);
        assert_eq!(edit.range, range(line, end, line, end));
        assert_generated_insert(&edit.new_text);
    }

    harness.finish();
}

pub(super) fn malformed_ids_receive_no_missing_id_action() {
    let mut harness = Harness::start();
    let source_uri = uri("file:///workspace/dialogue/code-action-malformed.recite");
    let source = concat!(
        ":: start default\n",
        "> @Greeting\n",
        "  Hello.\n",
        "? @ask speaker=rhea\n",
        "  Anywhere.\n",
    );
    harness.did_open(source_uri.clone(), 3, source);
    let _ = harness.recv_publish_diagnostics();

    for line in [1, 3] {
        let actions = code_actions(
            &mut harness,
            source_uri.clone(),
            range(line, 0, line, 1),
            None,
        );
        assert!(actions.is_empty(), "line {line}: {actions:?}");
    }

    harness.finish();
}

pub(super) fn source_fix_all_orders_deterministic_multi_edits_and_preserves_existing_ids() {
    let mut harness = Harness::start();
    let source_uri = uri("file:///workspace/dialogue/code-action-fix-all.recite");
    let source = concat!(
        ":: start default\n",
        "> @existing_line\n",
        "  Existing.\n",
        ">\n",
        "  Missing line.\n",
        "? @b2k59n3\n",
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
    for edit in &edits {
        assert_generated_insert(&edit.new_text);
    }
    let applied = apply_edits(source, &edits);
    assert!(applied.contains("> @existing_line\n"));
    assert!(applied.contains("? @b2k59n3\n"));

    harness.finish();
}

pub(super) fn source_fix_all_scopes_edits_and_guards_sibling_documents() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "a.recite", ":: a\n>\n  Missing.\n");
    write_file(temp.path(), "b.recite", ":: b\n> @ez1fh09\n  Existing.\n");
    let a_uri = file_uri(&temp.path().join("a.recite"));
    let b_uri = file_uri(&temp.path().join("b.recite"));
    let mut harness = harness_for_root(temp.path());
    harness.did_open(b_uri.clone(), 9, ":: b\n> @ez1fh09\n  Existing.\n");
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
    assert_generated_insert(&single_text_edit(&action).new_text);
    harness.finish();
}

pub(super) fn generated_ids_are_new_to_the_project() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let source_uri = file_uri(&temp.path().join("start.recite"));
    write_file(
        temp.path(),
        "start.recite",
        ":: start default\n>\n  Missing.\n",
    );
    write_file(
        temp.path(),
        "occupied.recite",
        ":: occupied\n? @k7q3m9x\n  Existing choice.\n",
    );

    let mut harness = harness_for_root(temp.path());
    let edit = fix_all(&mut harness, source_uri, range(0, 0, 2, 10));
    let edits = plain_text_edits(&edit);
    let [inserted] = edits.as_slice() else {
        panic!("expected one insertion");
    };
    assert_generated_insert(&inserted.new_text);
    assert_ne!(inserted_id(inserted), "@k7q3m9x");

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
    assert_eq!(text_edit.range, range(1, 13, 1, 13));
    assert_generated_insert(&text_edit.new_text);

    harness.finish();
}

pub(super) fn existing_ids_do_not_receive_missing_id_actions() {
    let mut harness = Harness::start();
    let source_uri = uri("file:///workspace/dialogue/code-action-existing.recite");
    let source = concat!(
        ":: start default\n",
        "> @hazel_rhea.small_talk\n",
        "  Existing author-chosen ID.\n",
        "? @hv6ceye\n",
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

/// Tooling appends exactly ` @id` with a freshly generated ID.
fn assert_generated_insert(insert: &str) {
    let Some(id) = insert.strip_prefix(" @") else {
        panic!("insert `{insert}` is not a trailing ID field");
    };
    assert_eq!(id.len(), GENERATED_STABLE_ID_LEN, "{id}");
    assert!(is_valid_stable_id(id), "{id}");
}
