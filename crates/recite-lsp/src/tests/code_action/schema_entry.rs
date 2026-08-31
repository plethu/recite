use lsp_types::CodeActionKind;
use tempfile::TempDir;

use super::support::{
    assert_no_action_title, code_actions, harness_for_root_with_schema_value, range,
    single_quick_fix_with_title, single_text_edit,
};
use crate::tests::support::{file_uri, full_change, write_file};

const SOURCE: &str = "schema_version = 1\n[producer]\nid = \"dialogue\"\n";

pub(super) fn condition_schema_quick_fix_inserts_zero_arg_bool_entry() {
    let (temp, source_uri, schema_uri) = fixture("schema.toml", SOURCE, ":if can_talk()\n");
    let mut harness = harness_for_root_with_schema_value(temp.path(), "./schema.toml");
    let edit = single_quick_fix_with_title(
        &mut harness,
        source_uri,
        range(1, 4, 1, 13),
        "Add condition `can_talk` to schema",
    );

    assert_eq!(edit.text_document.uri, schema_uri);
    assert_eq!(edit.text_document.version, None);
    let text = single_text_edit(&edit).new_text;
    assert!(text.contains("[conditions.can_talk]"));
    assert!(text.contains("returns = \"bool\""));
    assert!(text.contains("id = \"dialogue\""));
    harness.finish();
}

pub(super) fn condition_schema_quick_fix_rejects_arguments_and_match_scrutinee() {
    let (temp, source_uri, _) = fixture(
        "schema.toml",
        SOURCE,
        ":if has_flag(cell_key)\n:match thread_stage()\n  :case _\n    -> END\n",
    );
    let mut harness = harness_for_root_with_schema_value(temp.path(), "./schema.toml");
    let actions = code_actions(
        &mut harness,
        source_uri.clone(),
        range(1, 4, 1, 23),
        Some(vec![CodeActionKind::QUICKFIX]),
    );
    assert_no_action_title(&actions, "Add condition `has_flag` to schema");
    let actions = code_actions(
        &mut harness,
        source_uri,
        range(2, 7, 2, 20),
        Some(vec![CodeActionKind::QUICKFIX]),
    );
    assert_no_action_title(&actions, "Add condition `thread_stage` to schema");
    harness.finish();
}

pub(super) fn effect_schema_quick_fix_inserts_zero_arg_mode_entry() {
    let (temp, source_uri, _) = fixture("schema.toml", SOURCE, "! blocking mark_seen()\n");
    let mut harness = harness_for_root_with_schema_value(temp.path(), "./schema.toml");
    let edit = single_quick_fix_with_title(
        &mut harness,
        source_uri,
        range(1, 11, 1, 20),
        "Add effect `mark_seen` to schema",
    );
    let text = single_text_edit(&edit).new_text;
    assert!(text.contains("[effects.mark_seen]"));
    assert!(text.contains("modes = [\"blocking\"]"));
    harness.finish();
}

pub(super) fn effect_schema_quick_fix_rejects_arguments_and_metadata() {
    let (temp, source_uri, _) = fixture(
        "schema.toml",
        SOURCE,
        "! deferred advance_thread(thread)\n> mood=happy\n  Hello.\n",
    );
    let mut harness = harness_for_root_with_schema_value(temp.path(), "./schema.toml");
    let actions = code_actions(
        &mut harness,
        source_uri.clone(),
        range(1, 11, 1, 35),
        Some(vec![CodeActionKind::QUICKFIX]),
    );
    assert_no_action_title(&actions, "Add effect `advance_thread` to schema");
    let actions = code_actions(
        &mut harness,
        source_uri,
        range(2, 2, 2, 6),
        Some(vec![CodeActionKind::QUICKFIX]),
    );
    assert!(actions.iter().all(|action| {
        !matches!(action, lsp_types::CodeActionOrCommand::CodeAction(action)
            if action.title.starts_with("Add condition") || action.title.starts_with("Add effect"))
    }));
    harness.finish();
}

pub(super) fn schema_entry_quick_fix_uses_project_wide_same_name_function_context() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "schema.toml", SOURCE);
    write_file(
        temp.path(),
        "condition.recite",
        ":: start\n:if ready()\n  -> END\n",
    );
    write_file(
        temp.path(),
        "effect.recite",
        ":: start\n! deferred pulse()\n",
    );
    let effect_uri = file_uri(&temp.path().join("effect.recite"));
    let mut harness = harness_for_root_with_schema_value(temp.path(), "./schema.toml");
    let edit = single_quick_fix_with_title(
        &mut harness,
        effect_uri,
        range(1, 11, 1, 16),
        "Add effect `pulse` to schema",
    );
    assert!(single_text_edit(&edit).new_text.contains("[effects.pulse]"));
    harness.finish();
}

pub(super) fn schema_entry_quick_fix_rejects_incomplete_project_function_summaries() {
    let (temp, source_uri, _) = fixture("schema.toml", SOURCE, ":if ready(\n");
    let mut harness = harness_for_root_with_schema_value(temp.path(), "./schema.toml");
    let actions = code_actions(
        &mut harness,
        source_uri,
        range(1, 4, 1, 10),
        Some(vec![CodeActionKind::QUICKFIX]),
    );
    assert_no_action_title(&actions, "Add condition `ready` to schema");
    harness.finish();
}

pub(super) fn schema_entry_insertion_handles_crlf_and_eof_without_trailing_newline() {
    let (temp, source_uri, _) = fixture(
        "schema.toml",
        "schema_version = 1\r\n[producer]\r\nid = \"dialogue\"",
        ":if ready()\r\n  -> END\r\n",
    );
    let mut harness = harness_for_root_with_schema_value(temp.path(), "./schema.toml");
    let edit = single_quick_fix_with_title(
        &mut harness,
        source_uri,
        range(1, 4, 1, 11),
        "Add condition `ready` to schema",
    );
    let text = single_text_edit(&edit).new_text;
    assert!(text.contains("\r\n[conditions.ready]"));
    assert!(!text.ends_with('\n'));
    harness.finish();
}

pub(super) fn schema_entry_quick_fix_rejects_missing_sections() {
    let (temp, source_uri, _) = fixture("schema.toml", SOURCE, ":if ready()\n");
    let mut harness = harness_for_root_with_schema_value(temp.path(), "./schema.toml");
    let edit = single_quick_fix_with_title(
        &mut harness,
        source_uri,
        range(1, 4, 1, 11),
        "Add condition `ready` to schema",
    );
    assert!(
        single_text_edit(&edit)
            .new_text
            .contains("[conditions.ready]")
    );
    harness.finish();
}

pub(super) fn schema_entry_quick_fix_rejects_open_schema_buffers() {
    let (temp, source_uri, schema_uri) = fixture("schema.toml", SOURCE, ":if ready()\n");
    let mut harness = harness_for_root_with_schema_value(temp.path(), "./schema.toml");
    harness.did_open(schema_uri.clone(), 2, SOURCE);
    let _ = harness.recv_publish_diagnostics();
    let edit = single_quick_fix_with_title(
        &mut harness,
        source_uri,
        range(1, 4, 1, 11),
        "Add condition `ready` to schema",
    );
    assert_eq!(edit.text_document.uri, schema_uri);
    assert_eq!(edit.text_document.version, Some(2));
    harness.did_change(edit.text_document.uri, 1, vec![full_change(SOURCE)]);
    harness.finish();
}

pub(super) fn generated_json_schema_has_no_schema_edit_actions() {
    let (temp, source_uri, _) = fixture("schema.json", "{\"schema_version\":1}\n", ":if ready()\n");
    let mut harness = harness_for_root_with_schema_value(temp.path(), "./schema.json");
    let actions = code_actions(
        &mut harness,
        source_uri,
        range(0, 4, 0, 11),
        Some(vec![CodeActionKind::QUICKFIX]),
    );
    assert_no_action_title(&actions, "Add condition `ready` to schema");
    harness.finish();
}

pub(super) fn unknown_schema_extension_has_unavailable_edit_capability() {
    let (temp, source_uri, _) = fixture("schema.data", SOURCE, ":if ready()\n");
    let mut harness = harness_for_root_with_schema_value(temp.path(), "./schema.data");
    let actions = code_actions(
        &mut harness,
        source_uri,
        range(1, 4, 1, 11),
        Some(vec![CodeActionKind::QUICKFIX]),
    );
    assert_no_action_title(&actions, "Add condition `ready` to schema");
    harness.finish();
}

fn fixture(name: &str, schema: &str, scene: &str) -> (TempDir, lsp_types::Uri, lsp_types::Uri) {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), name, schema);
    write_file(temp.path(), "scene.recite", &format!(":: start\n{scene}"));
    let source_uri = file_uri(&temp.path().join("scene.recite"));
    let schema_uri = file_uri(&temp.path().join(name));
    (temp, source_uri, schema_uri)
}
