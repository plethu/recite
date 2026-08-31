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
    assert!(
        actions
            .iter()
            .any(|action| action["edit"]["documentChanges"].is_array()),
        "code action response omitted a source-preserving edit: {actions:?}"
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
