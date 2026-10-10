mod support;

use serde_json::{Value, json};
use support::stdio::{StdioHarness, file_uri};

#[test]
fn watched_batch_publishes_diagnostics_for_each_closed_source() {
    let directory = workspace().unwrap();
    let first = directory.path().join("first.recite");
    let second = directory.path().join("second.recite");
    let first_uri = file_uri(&first);
    let second_uri = file_uri(&second);
    let mut harness = StdioHarness::start(json!({
        "capabilities": {}, "rootUri": file_uri(directory.path())
    }));
    assert!(harness.barrier(&first_uri).is_empty());
    std::fs::write(&first, "oops\n:: first\n").unwrap();
    std::fs::write(&second, "oops\n:: second\n").unwrap();

    harness.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [
        { "uri": first_uri, "type": 2 },
        { "uri": second_uri, "type": 2 }
    ] }),
    );

    let messages = harness.barrier(&first_uri);
    assert_batch(&messages, &[(&first_uri, false), (&second_uri, false)]);
    for message in &messages {
        assert!(
            message["params"]["diagnostics"]
                .as_array()
                .unwrap()
                .iter()
                .any(|diagnostic| diagnostic["code"] == "RECITE_PARSE001")
        );
    }
    harness.finish();
}

#[test]
fn watched_batch_keeps_earlier_deletion_clear() {
    let directory = workspace().unwrap();
    let first = directory.path().join("first.recite");
    let second = directory.path().join("second.recite");
    let first_uri = file_uri(&first);
    let second_uri = file_uri(&second);
    let mut harness = StdioHarness::start(json!({
        "capabilities": {}, "rootUri": file_uri(directory.path())
    }));
    harness.barrier(&first_uri);
    std::fs::write(&first, "oops\n:: first\n").unwrap();
    harness.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [
        { "uri": first_uri, "type": 2 }
    ] }),
    );
    assert_batch(&harness.barrier(&first_uri), &[(&first_uri, false)]);
    std::fs::remove_file(&first).unwrap();
    std::fs::write(&second, "oops\n:: second\n").unwrap();

    harness.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [
        { "uri": first_uri, "type": 3 },
        { "uri": second_uri, "type": 2 }
    ] }),
    );

    assert_batch(
        &harness.barrier(&first_uri),
        &[(&first_uri, true), (&second_uri, false)],
    );
    harness.finish();
}

#[test]
fn watched_batch_keeps_manifest_schema_authority_transition() {
    let directory = workspace().unwrap();
    let manifest = directory.path().join("recite.project.toml");
    let schema_a = directory.path().join("schema-a.json");
    let schema_b = directory.path().join("schema-b.json");
    let source = directory.path().join("second.recite");
    std::fs::write(
        &manifest,
        "format_version = 1\n[project]\nschema = \"schema-a.json\"\n",
    )
    .unwrap();
    for schema in [&schema_a, &schema_b] {
        std::fs::write(schema, "{\"schema_version\":\"invalid\"}\n").unwrap();
    }
    let schema_a_uri = file_uri(&schema_a);
    let schema_b_uri = file_uri(&schema_b);
    let source_uri = file_uri(&source);
    let mut harness = StdioHarness::start(json!({
        "capabilities": {}, "rootUri": file_uri(directory.path())
    }));
    assert_batch(&harness.barrier(&source_uri), &[(&schema_a_uri, false)]);
    std::fs::write(
        &manifest,
        "format_version = 1\n[project]\nschema = \"schema-b.json\"\n",
    )
    .unwrap();
    std::fs::write(&source, "oops\n:: second\n").unwrap();

    harness.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [
        { "uri": file_uri(&manifest), "type": 2 },
        { "uri": source_uri, "type": 2 }
    ] }),
    );

    assert_batch(
        &harness.barrier(&source_uri),
        &[
            (&schema_a_uri, true),
            (&schema_b_uri, false),
            (&source_uri, false),
        ],
    );
    harness.finish();
}

fn workspace() -> std::io::Result<tempfile::TempDir> {
    let directory = tempfile::tempdir()?;
    std::fs::write(directory.path().join("first.recite"), ":: first\n")?;
    std::fs::write(directory.path().join("second.recite"), ":: second\n")?;
    Ok(directory)
}

fn assert_batch(messages: &[Value], expected: &[(&str, bool)]) {
    assert_eq!(
        messages.len(),
        expected.len(),
        "publication batch: {messages:?}"
    );
    for (message, (uri, clear)) in messages.iter().zip(expected) {
        assert_eq!(message["method"], "textDocument/publishDiagnostics");
        assert_eq!(message["params"]["uri"], *uri);
        assert!(message["params"]["version"].is_null());
        assert_eq!(
            message["params"]["diagnostics"]
                .as_array()
                .is_some_and(Vec::is_empty),
            *clear
        );
    }
}
