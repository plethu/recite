use lsp_types::CodeActionKind;
use tempfile::TempDir;

use super::support::{
    apply_edits, assert_no_action_title, code_actions, harness_for_root_with_schema,
    harness_for_root_with_schema_value, range, schema_manifest, single_quick_fix_with_title,
    single_text_edit,
};
use crate::tests::support::{file_uri, full_change, write_file};

pub(super) fn condition_schema_quick_fix_inserts_zero_arg_bool_entry() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(
        temp.path(),
        "schema.json",
        &schema_manifest("\"existing\": { \"params\": [] }", ""),
    );
    write_file(
        temp.path(),
        "scene.recite",
        ":: start default\n:if can_talk()\n  -> END\n",
    );
    let source_uri = file_uri(&temp.path().join("scene.recite"));
    let schema_uri = file_uri(&temp.path().join("schema.json"));
    let mut harness = harness_for_root_with_schema(temp.path());

    let edit = single_quick_fix_with_title(
        &mut harness,
        source_uri,
        range(1, 4, 1, 12),
        "Add condition `can_talk` to schema",
    );

    assert_eq!(edit.text_document.uri, schema_uri);
    assert_eq!(edit.text_document.version, None);
    let text_edit = single_text_edit(&edit);
    assert!(
        text_edit
            .new_text
            .contains(",\n    \"can_talk\": { \"params\": [] }\n  ")
    );
    let schema = schema_manifest("\"existing\": { \"params\": [] }", "");
    let applied = apply_edits(&schema, &[text_edit]);
    serde_json::from_str::<serde_json::Value>(&applied).expect("schema remains valid JSON");

    harness.finish();
}

pub(super) fn condition_schema_quick_fix_rejects_arguments_and_match_scrutinee() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "schema.json", &schema_manifest("", ""));
    write_file(
        temp.path(),
        "scene.recite",
        concat!(
            ":: start default\n",
            ":if has_flag(cell_key)\n",
            "  -> END\n",
            ":match thread_stage()\n",
            "  :case _\n",
            "    -> END\n",
        ),
    );
    let source_uri = file_uri(&temp.path().join("scene.recite"));
    let mut harness = harness_for_root_with_schema(temp.path());

    let with_args = code_actions(
        &mut harness,
        source_uri.clone(),
        range(1, 4, 1, 12),
        Some(vec![CodeActionKind::QUICKFIX]),
    );
    assert_no_action_title(&with_args, "Add condition `has_flag` to schema");

    let match_scrutinee = code_actions(
        &mut harness,
        source_uri,
        range(3, 7, 3, 19),
        Some(vec![CodeActionKind::QUICKFIX]),
    );
    assert_no_action_title(&match_scrutinee, "Add condition `thread_stage` to schema");

    harness.finish();
}

pub(super) fn effect_schema_quick_fix_inserts_zero_arg_mode_entry() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "schema.json", &schema_manifest("", ""));
    write_file(
        temp.path(),
        "scene.recite",
        ":: start default\n! blocking mark_seen()\n",
    );
    let source_uri = file_uri(&temp.path().join("scene.recite"));
    let schema_uri = file_uri(&temp.path().join("schema.json"));
    let mut harness = harness_for_root_with_schema(temp.path());

    let edit = single_quick_fix_with_title(
        &mut harness,
        source_uri,
        range(1, 11, 1, 20),
        "Add effect `mark_seen` to schema",
    );

    assert_eq!(edit.text_document.uri, schema_uri);
    assert_eq!(edit.text_document.version, None);
    let text_edit = single_text_edit(&edit);
    assert_eq!(
        text_edit.new_text,
        "\n    \"mark_seen\": { \"modes\": [\"blocking\"], \"params\": [] }\n  "
    );
    let schema = schema_manifest("", "");
    let applied = apply_edits(&schema, &[text_edit]);
    serde_json::from_str::<serde_json::Value>(&applied).expect("schema remains valid JSON");

    harness.finish();
}

pub(super) fn effect_schema_quick_fix_rejects_arguments_and_metadata() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "schema.json", &schema_manifest("", ""));
    write_file(
        temp.path(),
        "scene.recite",
        concat!(
            ":: start default\n",
            "! deferred advance_thread(thread)\n",
            "> mood=happy\n",
            "> speaker=rhea\n",
            "> cue=stare\n",
            "  Hello.\n",
        ),
    );
    let source_uri = file_uri(&temp.path().join("scene.recite"));
    let mut harness = harness_for_root_with_schema(temp.path());

    let effect_with_args = code_actions(
        &mut harness,
        source_uri.clone(),
        range(1, 11, 1, 25),
        Some(vec![CodeActionKind::QUICKFIX]),
    );
    assert_no_action_title(&effect_with_args, "Add effect `advance_thread` to schema");

    for metadata_range in [range(2, 2, 2, 6), range(3, 2, 3, 9), range(4, 2, 4, 5)] {
        let metadata = code_actions(
            &mut harness,
            source_uri.clone(),
            metadata_range,
            Some(vec![CodeActionKind::QUICKFIX]),
        );
        // The compiler-owned stable-ID action may overlap this range; the
        // schema action remains unavailable for metadata keys.
        for name in ["mood", "speaker", "cue"] {
            assert_no_action_title(&metadata, &format!("Add condition `{name}` to schema"));
            assert_no_action_title(&metadata, &format!("Add effect `{name}` to schema"));
        }
    }

    harness.finish();
}

pub(super) fn schema_entry_quick_fix_uses_project_wide_same_name_function_context() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "schema.json", &schema_manifest("", ""));
    write_file(
        temp.path(),
        "conditions_bool.recite",
        concat!(":: start default\n", ":if ready()\n", "  -> END\n",),
    );
    write_file(
        temp.path(),
        "conditions_match.recite",
        concat!(
            ":: condition_match\n",
            ":match ready()\n",
            "  :case _\n",
            "    -> END\n",
        ),
    );
    write_file(
        temp.path(),
        "effects_deferred.recite",
        concat!(":: effects\n", "! deferred pulse()\n"),
    );
    write_file(
        temp.path(),
        "effects_immediate.recite",
        concat!(":: effects_immediate\n", "! immediate pulse()\n",),
    );
    let condition_uri = file_uri(&temp.path().join("conditions_bool.recite"));
    let effect_uri = file_uri(&temp.path().join("effects_deferred.recite"));
    let mut harness = harness_for_root_with_schema(temp.path());

    let mixed_condition = code_actions(
        &mut harness,
        condition_uri,
        range(1, 4, 1, 9),
        Some(vec![CodeActionKind::QUICKFIX]),
    );
    assert_no_action_title(&mixed_condition, "Add condition `ready` to schema");

    let edit = single_quick_fix_with_title(
        &mut harness,
        effect_uri,
        range(1, 11, 1, 16),
        "Add effect `pulse` to schema",
    );
    let text_edit = single_text_edit(&edit);
    assert_eq!(
        text_edit.new_text,
        "\n    \"pulse\": { \"modes\": [\"deferred\", \"immediate\"], \"params\": [] }\n  "
    );

    harness.finish();
}

pub(super) fn schema_entry_quick_fix_rejects_incomplete_project_function_summaries() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "schema.json", &schema_manifest("", ""));
    write_file(
        temp.path(),
        "clean.recite",
        concat!(
            ":: start default\n",
            ":if ready()\n",
            "  ! deferred pulse()\n"
        ),
    );
    write_file(
        temp.path(),
        "broken.recite",
        concat!(":: broken\n", ":if ready(\n"),
    );
    let source_uri = file_uri(&temp.path().join("clean.recite"));
    let mut harness = harness_for_root_with_schema(temp.path());

    let condition = code_actions(
        &mut harness,
        source_uri.clone(),
        range(1, 4, 1, 9),
        Some(vec![CodeActionKind::QUICKFIX]),
    );
    assert_no_action_title(&condition, "Add condition `ready` to schema");

    let effect = code_actions(
        &mut harness,
        source_uri,
        range(2, 13, 2, 18),
        Some(vec![CodeActionKind::QUICKFIX]),
    );
    assert_no_action_title(&effect, "Add effect `pulse` to schema");

    harness.finish();
}

pub(super) fn schema_entry_insertion_handles_crlf_and_eof_without_trailing_newline() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let schema = "{\r\n  \"schema_version\": 1,\r\n  \"conditions\": {},\r\n  \"effects\": {}\r\n}";
    write_file(temp.path(), "schema.json", schema);
    write_file(
        temp.path(),
        "scene.recite",
        ":: start default\n:if ready()\n  -> END\n",
    );
    let source_uri = file_uri(&temp.path().join("scene.recite"));
    let mut harness = harness_for_root_with_schema(temp.path());

    let edit = single_quick_fix_with_title(
        &mut harness,
        source_uri,
        range(1, 4, 1, 9),
        "Add condition `ready` to schema",
    );

    let text_edit = single_text_edit(&edit);
    assert_eq!(
        text_edit.new_text,
        "\r\n    \"ready\": { \"params\": [] }\r\n  "
    );
    let applied = apply_edits(schema, &[text_edit]);
    serde_json::from_str::<serde_json::Value>(&applied).expect("schema remains valid JSON");

    harness.finish();
}

pub(super) fn schema_entry_quick_fix_rejects_missing_sections() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(
        temp.path(),
        "schema.json",
        "{\n  \"schema_version\": 1\n}\n",
    );
    write_file(
        temp.path(),
        "scene.recite",
        ":: start default\n:if ready()\n  -> END\n",
    );
    let source_uri = file_uri(&temp.path().join("scene.recite"));
    let mut harness = harness_for_root_with_schema(temp.path());

    let missing_section = code_actions(
        &mut harness,
        source_uri.clone(),
        range(1, 4, 1, 9),
        Some(vec![CodeActionKind::QUICKFIX]),
    );
    assert_no_action_title(&missing_section, "Add condition `ready` to schema");

    harness.finish();
}

pub(super) fn schema_entry_quick_fix_rejects_open_schema_buffers() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "schema.json", &schema_manifest("", ""));
    write_file(
        temp.path(),
        "scene.recite",
        ":: start default\n:if ready()\n  -> END\n",
    );
    let source_uri = file_uri(&temp.path().join("scene.recite"));
    let schema_uri = file_uri(&temp.path().join("schema.json"));
    let mut harness = harness_for_root_with_schema_value(temp.path(), "./schema.json");

    let would_offer = code_actions(
        &mut harness,
        source_uri.clone(),
        range(1, 4, 1, 9),
        Some(vec![CodeActionKind::QUICKFIX]),
    );
    assert!(
        would_offer
            .iter()
            .any(|action| matches!(action, lsp_types::CodeActionOrCommand::CodeAction(action) if action.title == "Add condition `ready` to schema")),
        "saved schema should offer a condition action before schema buffer opens"
    );

    harness.did_open(schema_uri.clone(), 2, &schema_manifest("", ""));
    let _ = harness.recv_publish_diagnostics();
    harness.did_change(
        schema_uri,
        3,
        vec![full_change(&schema_manifest(
            "\"other\": { \"params\": [] }",
            "",
        ))],
    );
    let _ = harness.recv_publish_diagnostics();

    let open_schema = code_actions(
        &mut harness,
        source_uri,
        range(1, 4, 1, 9),
        Some(vec![CodeActionKind::QUICKFIX]),
    );
    assert_no_action_title(&open_schema, "Add condition `ready` to schema");

    harness.finish();
}
