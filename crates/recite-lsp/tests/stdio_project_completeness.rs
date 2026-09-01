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
        "textDocument/didSave",
        json!({"textDocument": {"uri": omitted_uri.clone()}}),
    );
    let complete = harness.barrier(&source_uri);
    assert_publish_batch(
        &complete,
        &[
            (&manifest_uri, None, &[]),
            (&omitted_uri, None, &[]),
            (
                &source_uri,
                Some(1),
                &["RECITE_ID001", "RECITE_VALIDATE007"],
            ),
        ],
    );

    std::fs::write(&omitted, [b':', b':', b' ', b'\xfe'])
        .unwrap_or_else(|error| panic!("make source incomplete: {error}"));
    harness.notify(
        "textDocument/didSave",
        json!({"textDocument": {"uri": omitted_uri.clone()}}),
    );
    let incomplete = harness.barrier(&source_uri);
    assert_publish_batch(
        &incomplete,
        &[
            (&manifest_uri, None, &["RECITE_CONFIG115"]),
            (&omitted_uri, None, &[]),
            (&source_uri, Some(1), &["RECITE_ID001"]),
        ],
    );
    harness.finish();
}

fn assert_publish_batch(messages: &[Value], expected: &[(&str, Option<i64>, &[&str])]) {
    assert_eq!(
        messages.len(),
        expected.len(),
        "unexpected messages in diagnostic batch: {messages:?}"
    );
    let published = messages
        .iter()
        .filter(|message| message["method"] == "textDocument/publishDiagnostics")
        .collect::<Vec<_>>();
    assert_eq!(
        published.len(),
        expected.len(),
        "unexpected diagnostic batch: {messages:?}"
    );
    for (message, (uri, version, codes)) in published.iter().zip(expected) {
        assert_eq!(message["params"]["uri"], *uri);
        assert_eq!(message["params"]["version"].as_i64(), *version);
        let actual_codes = message["params"]["diagnostics"]
            .as_array()
            .unwrap_or_else(|| panic!("diagnostic array for {uri}: {message}"))
            .iter()
            .map(|diagnostic| {
                diagnostic["code"]
                    .as_str()
                    .unwrap_or_else(|| panic!("diagnostic code for {uri}: {diagnostic}"))
            })
            .collect::<Vec<_>>();
        assert_eq!(actual_codes, *codes, "diagnostics for {uri}: {message}");
    }
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
