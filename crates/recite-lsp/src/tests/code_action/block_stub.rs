use lsp_types::CodeActionKind;
use tempfile::TempDir;

use super::support::{
    apply_edits, assert_no_action_title, code_actions, range, single_quick_fix_with_title,
    single_text_edit,
};
use crate::tests::support::{Harness, file_uri, harness_for_root, uri, write_file};

pub(super) fn block_stub_quick_fix_inserts_local_eof_stub() {
    let mut harness = Harness::start();
    let source_uri = uri("file:///workspace/dialogue/block-stub.recite");
    let source = concat!(":: start default\n", "-> missing_block\n");
    harness.did_open(source_uri.clone(), 11, source);
    let _ = harness.recv_publish_diagnostics();

    let edit = single_quick_fix_with_title(
        &mut harness,
        source_uri,
        range(1, 3, 1, 16),
        "Create block stub `missing_block`",
    );

    assert_eq!(edit.text_document.version, Some(11));
    let text_edit = single_text_edit(&edit);
    assert_eq!(text_edit.range, range(2, 0, 2, 0));
    assert_eq!(text_edit.new_text, ":: missing_block\n");
    assert!(apply_edits(source, &[text_edit]).ends_with(":: missing_block\n"));

    harness.finish();
}

pub(super) fn block_stub_quick_fix_targets_unique_external_file() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(
        temp.path(),
        "dialogue/start.recite",
        ":: start default\n-> dialogue/next.recite::later\n",
    );
    write_file(temp.path(), "dialogue/next.recite", ":: next\n");
    let source_uri = file_uri(&temp.path().join("dialogue/start.recite"));
    let target_uri = file_uri(&temp.path().join("dialogue/next.recite"));
    let mut harness = harness_for_root(temp.path());

    let edit = single_quick_fix_with_title(
        &mut harness,
        source_uri,
        range(1, 24, 1, 29),
        "Create block stub `later`",
    );

    assert_eq!(edit.text_document.uri, target_uri);
    assert_eq!(edit.text_document.version, None);
    let text_edit = single_text_edit(&edit);
    assert_eq!(text_edit.range, range(1, 0, 1, 0));
    assert_eq!(text_edit.new_text, ":: later\n");

    harness.finish();
}

pub(super) fn block_stub_quick_fix_rejects_unresolved_target_and_target_collision() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(
        temp.path(),
        "dialogue/start.recite",
        ":: start default\n-> dialogue/missing.recite::later\n-> local_later\n-> dialogue/other.recite::local_later\n",
    );
    write_file(temp.path(), "dialogue/other.recite", ":: local_later\n");
    let source_uri = file_uri(&temp.path().join("dialogue/start.recite"));
    let mut harness = harness_for_root(temp.path());

    let unresolved = code_actions(
        &mut harness,
        source_uri.clone(),
        range(1, 28, 1, 33),
        Some(vec![CodeActionKind::QUICKFIX]),
    );
    assert_no_action_title(&unresolved, "Create block stub `later`");

    let other_file_collision = single_quick_fix_with_title(
        &mut harness,
        source_uri.clone(),
        range(2, 3, 2, 14),
        "Create block stub `local_later`",
    );
    assert_eq!(other_file_collision.text_document.uri, source_uri.clone());

    let target_collision = code_actions(
        &mut harness,
        source_uri,
        range(3, 27, 3, 38),
        Some(vec![CodeActionKind::QUICKFIX]),
    );
    assert_no_action_title(&target_collision, "Create block stub `local_later`");

    harness.finish();
}

pub(super) fn block_stub_quick_fix_rejects_incomplete_block_reference_summary() {
    let mut harness = Harness::start();
    let source_uri = uri("file:///workspace/dialogue/incomplete-block-ref.recite");
    let source = concat!(":: start default\n", "-> missing_block\n", ":if\n");
    harness.did_open(source_uri.clone(), 7, source);
    let _ = harness.recv_publish_diagnostics();

    let actions = code_actions(
        &mut harness,
        source_uri,
        range(1, 3, 1, 16),
        Some(vec![CodeActionKind::QUICKFIX]),
    );
    assert_no_action_title(&actions, "Create block stub `missing_block`");

    harness.finish();
}
