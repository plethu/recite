mod support;

use serde_json::{Value, json};
use tempfile::Builder;

use support::stdio::{StdioHarness, file_uri};

#[test]
fn non_utf8_discovery_suppresses_cross_file_ids_and_republishes_transitions() {
    let temp = Builder::new()
        .prefix("recite project completeness ")
        .tempdir()
        .unwrap_or_else(|error| panic!("temporary workspace: {error}"));
    let manifest = temp.path().join("recite.project.toml");
    let source = temp.path().join("src/main.recite");
    let omitted = temp.path().join("src/omitted.recite");
    std::fs::create_dir_all(omitted.parent().unwrap_or_else(|| panic!("source parent")))
        .unwrap_or_else(|error| panic!("create source root: {error}"));
    std::fs::write(
        &manifest,
        "format_version = 1\n[discovery]\nsource_roots = [\"src\"]\n",
    )
    .unwrap_or_else(|error| panic!("write manifest: {error}"));
    std::fs::write(
        &source,
        ":: main default\n>\n  Missing line id.\n-> src/omitted.recite::missing\n",
    )
    .unwrap_or_else(|error| panic!("write source: {error}"));
    std::fs::write(&omitted, [b':', b':', b' ', b'\xff'])
        .unwrap_or_else(|error| panic!("write incomplete source: {error}"));

    let manifest_uri = file_uri(&manifest);
    let source_uri = file_uri(&source);
    let omitted_uri = file_uri(&omitted);
    let mut harness = StdioHarness::start(json!({
        "capabilities": {},
        "rootUri": file_uri(temp.path())
    }));
    harness.notify(
        "textDocument/didOpen",
        json!({"textDocument": {"uri": source_uri.clone(), "languageId": "recite", "version": 1, "text": ":: main default\n>\n  Missing line id.\n-> src/omitted.recite::missing\n"}}),
    );
    let initial = harness.barrier(&source_uri);
    let initial_source = diagnostics_for(&initial, &source_uri);
    assert!(
        initial_source
            .iter()
            .any(|diagnostic| diagnostic["code"] == "RECITE_ID001")
    );
    assert!(
        !initial_source
            .iter()
            .any(|diagnostic| diagnostic["code"] == "RECITE_VALIDATE007")
    );
    assert!(
        diagnostics_for(&initial, &manifest_uri)
            .iter()
            .any(|diagnostic| diagnostic["code"] == "RECITE_CONFIG115")
    );

    std::fs::write(&omitted, ":: target\n")
        .unwrap_or_else(|error| panic!("restore complete source: {error}"));
    harness.notify(
        "workspace/didChangeWatchedFiles",
        json!({"changes": [{"uri": omitted_uri.clone(), "type": 2}]}),
    );
    let complete = harness.barrier(&source_uri);
    let complete_source = diagnostics_for(&complete, &source_uri);
    assert!(
        complete_source
            .iter()
            .any(|diagnostic| diagnostic["code"] == "RECITE_ID001")
    );
    assert!(
        complete_source
            .iter()
            .any(|diagnostic| diagnostic["code"] == "RECITE_VALIDATE007")
    );
    assert!(diagnostics_for(&complete, &manifest_uri).is_empty());

    std::fs::write(&omitted, [b':', b':', b' ', b'\xfe'])
        .unwrap_or_else(|error| panic!("make source incomplete: {error}"));
    harness.notify(
        "workspace/didChangeWatchedFiles",
        json!({"changes": [{"uri": omitted_uri, "type": 2}]}),
    );
    let incomplete = harness.barrier(&source_uri);
    let incomplete_source = diagnostics_for(&incomplete, &source_uri);
    assert!(
        incomplete_source
            .iter()
            .any(|diagnostic| diagnostic["code"] == "RECITE_ID001")
    );
    assert!(
        !incomplete_source
            .iter()
            .any(|diagnostic| diagnostic["code"] == "RECITE_VALIDATE007")
    );
    assert!(
        diagnostics_for(&incomplete, &manifest_uri)
            .iter()
            .any(|diagnostic| diagnostic["code"] == "RECITE_CONFIG115")
    );
    harness.finish();
}

fn diagnostics_for<'a>(messages: &'a [Value], uri: &str) -> Vec<&'a Value> {
    messages
        .iter()
        .filter(|message| {
            message["method"] == "textDocument/publishDiagnostics"
                && message["params"]["uri"] == uri
        })
        .flat_map(|message| {
            message["params"]["diagnostics"]
                .as_array()
                .into_iter()
                .flatten()
        })
        .collect()
}
