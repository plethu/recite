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
const MALFORMED_FIXTURE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/recite/invalid/parser_marker_leading_prose.recite"
));

#[test]
fn initialize_and_project_features_use_shared_stdio_contract() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("temporary project: {error}"));
    let source = write_source(temp.path(), "dialogue.recite", CORE_FIXTURE);
    let uri = file_uri(&source);
    let mut harness = start_harness(temp.path());

    let capabilities = &harness.initialize()["capabilities"];
    assert_eq!(capabilities["positionEncoding"], "utf-16");
    assert_eq!(capabilities["textDocumentSync"]["change"], 1);
    assert_eq!(capabilities["textDocumentSync"]["openClose"], true);
    assert!(capabilities["textDocumentSync"]["save"].is_object());
    assert!(capabilities["completionProvider"].is_object());
    assert_eq!(capabilities["hoverProvider"], true);
    assert_eq!(capabilities["definitionProvider"], true);
    assert_eq!(capabilities["referencesProvider"], true);
    assert_eq!(capabilities["renameProvider"]["prepareProvider"], true);
    assert!(capabilities["codeActionProvider"].is_object());

    harness.did_open(&uri, 1, CORE_FIXTURE);
    let diagnostics = harness.diagnostics(&uri);
    assert_eq!(diagnostics["version"], 1);
    assert!(diagnostics["diagnostics"].as_array().is_some());

    let completion = harness.request_result(
        "textDocument/completion",
        json!({
            "textDocument": { "uri": uri },
            "position": position_after(CORE_FIXTURE, "-> wo")
        }),
    );
    let labels = completion
        .as_array()
        .unwrap_or_else(|| panic!("completion result is not an array: {completion}"));
    assert!(
        labels.iter().any(|item| item["label"] == "work"),
        "shared completion omitted work: {completion}"
    );

    let target_position = position_for_byte_index(
        CORE_FIXTURE,
        CORE_FIXTURE
            .find("-> work")
            .unwrap_or_else(|| panic!("work target"))
            + 4,
    );
    let definition = harness.request_result(
        "textDocument/definition",
        json!({
            "textDocument": { "uri": uri },
            "position": target_position
        }),
    );
    assert_eq!(definition["uri"], uri);
    assert_eq!(definition["range"]["start"]["line"], 13);
    assert_eq!(definition["range"]["start"]["character"], 3);

    let hover = harness.request_result(
        "textDocument/hover",
        json!({ "textDocument": { "uri": uri }, "position": target_position }),
    );
    assert_eq!(hover["range"]["start"]["line"], 6);
    assert!(hover["contents"].is_object() || hover["contents"].is_array());

    let references = harness.request_result(
        "textDocument/references",
        json!({
            "textDocument": { "uri": uri },
            "position": target_position,
            "context": { "includeDeclaration": true }
        }),
    );
    assert_eq!(references.as_array().map(Vec::len), Some(2));
    assert_eq!(references[0]["range"]["start"]["line"], 13);
    assert_eq!(references[1]["range"]["start"]["line"], 6);

    let prepare_rename = harness.request_result(
        "textDocument/prepareRename",
        json!({ "textDocument": { "uri": uri }, "position": target_position }),
    );
    assert_eq!(prepare_rename["placeholder"], "work");
    let rename = harness.request_result(
        "textDocument/rename",
        json!({
            "textDocument": { "uri": uri },
            "position": target_position,
            "newName": "finished"
        }),
    );
    assert_eq!(rename["documentChanges"].as_array().map(Vec::len), Some(1));
    harness.finish();
}

#[test]
fn diagnostic_recovery_keeps_incomplete_overlay_editable() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("temporary project: {error}"));
    let source = write_source(temp.path(), "dialogue.recite", MALFORMED_FIXTURE);
    let uri = file_uri(&source);
    let mut harness = start_harness(temp.path());

    harness.did_open(&uri, 1, MALFORMED_FIXTURE);
    let malformed = harness.diagnostics(&uri);
    assert_eq!(malformed["version"], 1);
    assert!(
        malformed["diagnostics"]
            .as_array()
            .is_some_and(
                |diagnostics| diagnostics.iter().any(|diagnostic| diagnostic["code"]
                    .as_str()
                    .is_some_and(|code| { code.starts_with("RECITE_PARSE") }))
            ),
        "malformed fixture did not publish a stable parse diagnostic: {malformed}"
    );
    assert_eq!(malformed["diagnostics"][0]["severity"], 1);
    assert_eq!(malformed["diagnostics"][0]["range"]["start"]["line"], 2);
    assert_eq!(
        malformed["diagnostics"][0]["range"]["start"]["character"],
        11
    );
    assert_eq!(malformed["diagnostics"][0]["range"]["end"]["character"], 13);
    assert_eq!(malformed["diagnostics"][1]["severity"], 1);
    assert_eq!(malformed["diagnostics"][1]["range"]["start"]["line"], 3);
    assert_eq!(
        malformed["diagnostics"][1]["range"]["start"]["character"],
        11
    );
    assert_eq!(malformed["diagnostics"][1]["range"]["end"]["character"], 13);

    harness.did_change(&uri, 2, ":: marker_probe default\n>");
    let incomplete = harness.diagnostics(&uri);
    assert_eq!(incomplete["version"], 2);
    assert!(
        !incomplete["diagnostics"]
            .as_array()
            .is_some_and(Vec::is_empty)
    );

    harness.did_change(&uri, 3, CORE_FIXTURE);
    let recovered = harness.diagnostics(&uri);
    assert_eq!(recovered["version"], 3);
    assert!(
        recovered["diagnostics"]
            .as_array()
            .is_some_and(Vec::is_empty)
    );
    harness.assert_no_stale_publication(&uri);
    harness.finish();
}

#[test]
fn utf16_crlf_and_non_bmp_ranges_are_projected_over_stdio() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("temporary project: {error}"));
    let transformed = MALFORMED_FIXTURE
        .replace("-> East", "-> 😀East")
        .replace('\n', "\r\n");
    let source = write_source(temp.path(), "unicode.recite", &transformed);
    let uri = file_uri(&source);
    let mut harness = start_harness(temp.path());

    harness.did_open(&uri, 1, &transformed);
    let diagnostics = harness.diagnostics(&uri);
    let diagnostic = diagnostics["diagnostics"]
        .as_array()
        .and_then(|diagnostics| {
            diagnostics
                .iter()
                .find(|diagnostic| diagnostic["range"]["start"]["line"] == 2)
        })
        .unwrap_or_else(|| {
            panic!("unicode fixture did not publish a line-2 diagnostic: {diagnostics}")
        });
    assert_eq!(diagnostic["severity"], 1);
    assert_eq!(diagnostic["range"]["start"]["character"], 13);
    assert_eq!(diagnostic["range"]["end"]["character"], 15);
    let following = diagnostics["diagnostics"]
        .as_array()
        .and_then(|diagnostics| {
            diagnostics
                .iter()
                .find(|diagnostic| diagnostic["range"]["start"]["line"] == 3)
        })
        .unwrap_or_else(|| {
            panic!("unicode fixture did not publish the following diagnostic: {diagnostics}")
        });
    assert_eq!(following["range"]["start"]["character"], 11);
    assert_eq!(following["range"]["end"]["character"], 13);
    harness.finish();
}

fn start_harness(root: &Path) -> StdioHarness {
    StdioHarness::start(json!({
        "capabilities": {
            "general": { "positionEncodings": ["utf-16"] }
        },
        "rootUri": file_uri(root)
    }))
}

fn write_source(root: &Path, name: &str, source: &str) -> PathBuf {
    let path = root.join(name);
    std::fs::create_dir_all(path.parent().unwrap_or(root))
        .unwrap_or_else(|error| panic!("create parent for {}: {error}", path.display()));
    std::fs::write(&path, source)
        .unwrap_or_else(|error| panic!("write {}: {error}", path.display()));
    path
}

fn file_uri(path: &Path) -> String {
    Url::from_file_path(path)
        .unwrap_or_else(|()| panic!("path cannot become a file URI: {}", path.display()))
        .to_string()
}

fn position_after(source: &str, needle: &str) -> Value {
    position_for_byte_index(
        source,
        source
            .find(needle)
            .unwrap_or_else(|| panic!("needle not found: {needle}"))
            + needle.len(),
    )
}

fn position_for_byte_index(source: &str, byte_index: usize) -> Value {
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
