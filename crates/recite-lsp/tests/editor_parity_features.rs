use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use tempfile::TempDir;
use url::Url;

mod support;
use support::stdio::StdioHarness;

const CORE_FIXTURE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/recite/valid/core_language_spike.recite"
));
const PRESSURE_FIXTURE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/recite/valid/language_pressure.recite"
));

#[test]
fn code_actions_project_stable_id_repairs_from_the_shared_kernel() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("temporary project: {error}"));
    let source_text = CORE_FIXTURE.replace(
        "> intro_001@637b1854a7f3ed42f045 speaker=hazel mood=calm mood=alert",
        ">",
    );
    let source = write_source(temp.path(), "dialogue.recite", &source_text);
    let uri = file_uri(&source);
    let mut harness = start_harness(temp.path());
    assert!(harness.initialize()["capabilities"]["codeActionProvider"].is_object());

    harness.did_open(&uri, 1, &source_text);
    let _ = harness.diagnostics(&uri);
    harness.did_change(&uri, 2, &source_text);
    let diagnostics = harness.diagnostics(&uri);
    let diagnostic_list = diagnostics["diagnostics"]
        .as_array()
        .unwrap_or_else(|| panic!("missing diagnostics array: {diagnostics}"));
    assert!(
        !diagnostic_list.is_empty(),
        "missing-id derivation had no diagnostics"
    );
    assert!(diagnostic_list.iter().any(|diagnostic| {
        diagnostic["code"] == "RECITE_ID001"
            && diagnostic["range"]
                == json!({
                    "start": { "line": 2, "character": 0 },
                    "end": { "line": 2, "character": 0 }
                })
    }));
    let actions = harness.request_result(
        "textDocument/codeAction",
        json!({
            "textDocument": { "uri": uri },
            "range": { "start": { "line": 2, "character": 0 }, "end": { "line": 2, "character": 1 } },
            "context": { "diagnostics": diagnostic_list, "only": ["quickfix"] }
        }),
    );
    let actions = actions
        .as_array()
        .unwrap_or_else(|| panic!("code action result is not an array: {actions}"));
    let action = actions
        .iter()
        .find(|action| {
            action["diagnostics"].as_array().is_some_and(|diagnostics| {
                diagnostics
                    .iter()
                    .any(|diagnostic| diagnostic["code"] == "RECITE_ID001")
            })
        })
        .unwrap_or_else(|| panic!("code action response omitted RECITE_ID001: {actions:?}"));
    let changes = action["edit"]["documentChanges"]
        .as_array()
        .unwrap_or_else(|| panic!("code action omitted documentChanges: {action}"));
    assert_eq!(changes.len(), 1);
    assert_eq!(
        changes[0]["textDocument"],
        json!({ "uri": uri, "version": 2 })
    );
    assert_eq!(
        changes[0]["edits"],
        json!([
            {
                "range": {
                    "start": { "line": 2, "character": 1 },
                    "end": { "line": 2, "character": 1 }
                },
                "newText": " line@fca25e3a4f53bebbc182"
            }
        ])
    );
    let applied = apply_text_edits(&source_text, &changes[0]["edits"]);
    assert_eq!(
        applied,
        source_text.replacen(">\n", "> line@fca25e3a4f53bebbc182\n", 1)
    );
    harness.finish();
}

#[test]
fn project_root_discovers_canonical_multi_file_overlays_for_navigation() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("temporary project: {error}"));
    let source_text = CORE_FIXTURE.replace("-> work", "-> pressure.recite::letters");
    let source = write_source(temp.path(), "dialogue.recite", &source_text);
    let pressure = write_source(temp.path(), "pressure.recite", PRESSURE_FIXTURE);
    let uri = file_uri(&source);
    let pressure_uri = file_uri(&pressure);
    let mut harness = start_harness(temp.path());

    harness.did_open(&uri, 1, &source_text);
    let diagnostics = harness.diagnostics(&uri);
    assert!(
        diagnostics["diagnostics"]
            .as_array()
            .is_some_and(Vec::is_empty)
    );

    let definition = harness.request_result(
        "textDocument/definition",
        json!({
            "textDocument": { "uri": uri },
            "position": position_after(&source_text, "-> pressure.recite::le")
        }),
    );
    assert_eq!(definition["uri"], pressure_uri);
    assert_eq!(definition["range"]["start"]["line"], 9);
    assert_eq!(definition["range"]["start"]["character"], 3);
    harness.finish();
}

#[test]
fn stale_version_is_refused_after_a_deterministic_stdio_barrier() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("temporary project: {error}"));
    let source = write_source(temp.path(), "dialogue.recite", CORE_FIXTURE);
    let uri = file_uri(&source);
    let mut harness = start_harness(temp.path());

    harness.did_open(&uri, 1, CORE_FIXTURE);
    let _ = harness.diagnostics(&uri);
    harness.did_change(&uri, 2, ":: incomplete default\n->");
    let _ = harness.diagnostics(&uri);
    harness.did_change(&uri, 3, CORE_FIXTURE);
    let current = harness.diagnostics(&uri);
    assert_eq!(current["version"], 3);

    harness.did_change(&uri, 2, "oops");
    let completion = harness.request_result(
        "textDocument/completion",
        json!({
            "textDocument": { "uri": uri },
            "position": position_after(CORE_FIXTURE, "-> wo")
        }),
    );
    harness.assert_no_stale_publication(&uri);
    assert!(
        completion
            .as_array()
            .is_some_and(|items| items.iter().any(|item| item["label"] == "work")),
        "stale update replaced the current overlay: {completion}"
    );
    harness.finish();
}

fn write_source(root: &Path, name: &str, source: &str) -> PathBuf {
    let path = root.join(name);
    std::fs::create_dir_all(path.parent().unwrap_or(root))
        .unwrap_or_else(|error| panic!("create parent for {}: {error}", path.display()));
    std::fs::write(&path, source)
        .unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
    path
}

fn start_harness(root: &Path) -> StdioHarness {
    StdioHarness::start(json!({
        "capabilities": {
            "general": { "positionEncodings": ["utf-16"] }
        },
        "rootUri": file_uri(root)
    }))
}

fn file_uri(path: &Path) -> String {
    Url::from_file_path(path)
        .unwrap_or_else(|()| panic!("path cannot become a file URI: {}", path.display()))
        .to_string()
}

fn position_after(source: &str, needle: &str) -> Value {
    let byte_index = source
        .find(needle)
        .unwrap_or_else(|| panic!("needle not found: {needle}"))
        + needle.len();
    let mut line = 0_u32;
    let mut character = 0_u32;
    for value in source[..byte_index].chars() {
        if value == '\n' {
            line = line.saturating_add(1);
            character = 0;
        } else {
            character = character.saturating_add(value.len_utf16() as u32);
        }
    }
    json!({ "line": line, "character": character })
}

fn apply_text_edits(source: &str, edits: &Value) -> String {
    let edits = edits
        .as_array()
        .unwrap_or_else(|| panic!("text edits are not an array: {edits}"));
    let mut output = source.to_owned();
    for edit in edits.iter().rev() {
        let range = &edit["range"];
        let start = byte_offset_for_position(&output, &range["start"]);
        let end = byte_offset_for_position(&output, &range["end"]);
        let replacement = edit["newText"]
            .as_str()
            .unwrap_or_else(|| panic!("text edit replacement is not a string: {edit}"));
        output.replace_range(start..end, replacement);
    }
    output
}

fn byte_offset_for_position(source: &str, position: &Value) -> usize {
    let line = position["line"]
        .as_u64()
        .unwrap_or_else(|| panic!("position line is not an integer: {position}"))
        as usize;
    let character = position["character"]
        .as_u64()
        .unwrap_or_else(|| panic!("position character is not an integer: {position}"))
        as u32;
    let mut offset = 0;
    for (line_index, value) in source.split_inclusive('\n').enumerate() {
        if line_index == line {
            let without_newline = value.strip_suffix('\n').unwrap_or(value);
            let mut utf16 = 0_u32;
            for (byte_index, scalar) in without_newline.char_indices() {
                if utf16 >= character {
                    return offset + byte_index;
                }
                utf16 = utf16.saturating_add(scalar.len_utf16() as u32);
            }
            return offset + without_newline.len();
        }
        offset += value.len();
    }
    source.len()
}
