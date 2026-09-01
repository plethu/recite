mod support;

use serde_json::{Value, json};
use tempfile::Builder;

use support::stdio::{StdioHarness, file_uri};

#[cfg(unix)]
#[test]
fn retired_symlink_alias_keeps_target_excluded_until_final_close() {
    use std::os::unix::fs::symlink;

    let temp = Builder::new()
        .prefix("recite retired symlink schema ")
        .tempdir()
        .unwrap_or_else(|error| panic!("temporary schema directory: {error}"));
    let manifest = temp.path().join("recite.project.toml");
    let target = temp.path().join("schema-old.json");
    let alias = temp.path().join("z-schema.json");
    let replacement = temp.path().join("schema-new.json");
    std::fs::write(
        &manifest,
        "format_version = 1\n[project]\nschema = \"schema-old.json\"\n",
    )
    .unwrap_or_else(|error| panic!("write manifest: {error}"));
    std::fs::write(&target, "{\"schema_version\":1}\n")
        .unwrap_or_else(|error| panic!("write target: {error}"));
    std::fs::write(&replacement, "{\"schema_version\":\"new\"}\n")
        .unwrap_or_else(|error| panic!("write replacement: {error}"));
    symlink(&target, &alias).unwrap_or_else(|error| panic!("schema symlink: {error}"));

    let target_uri = file_uri(&target);
    let alias_uri = file_uri(&alias);
    let replacement_uri = file_uri(&replacement);
    let manifest_uri = file_uri(&manifest);
    let mut harness = StdioHarness::start(json!({
        "capabilities": {},
        "rootUri": file_uri(temp.path())
    }));
    harness.notify(
        "textDocument/didOpen",
        json!({"textDocument": {"uri": target_uri, "languageId": "json", "version": 7, "text": "{\"schema_version\":\"bad\"}\n"}}),
    );
    let opened_target = harness.barrier(&target_uri);
    assert_publish_batch(&opened_target, &[(&target_uri, Some(7), false)]);
    harness.notify(
        "textDocument/didOpen",
        json!({"textDocument": {"uri": alias_uri, "languageId": "json", "version": 8, "text": "{\"schema_version\":1}\n"}}),
    );
    assert!(harness.barrier(&alias_uri).is_empty());

    std::fs::write(
        &manifest,
        "format_version = 1\n[project]\nschema = \"schema-new.json\"\n",
    )
    .unwrap_or_else(|error| panic!("switch schema target: {error}"));
    harness.notify(
        "workspace/didChangeWatchedFiles",
        json!({"changes": [{"uri": manifest_uri, "type": 2}]}),
    );
    let switched = harness.barrier(&target_uri);
    assert_publish_batch(
        &switched,
        &[
            (&target_uri, Some(7), true),
            (&alias_uri, Some(8), true),
            (&replacement_uri, None, false),
        ],
    );

    harness.notify(
        "textDocument/didClose",
        json!({"textDocument": {"uri": target_uri}}),
    );
    let closed_target = harness.barrier(&target_uri);
    assert_publish_batch(
        &closed_target,
        &[(&target_uri, None, true), (&alias_uri, Some(8), true)],
    );
    harness.notify(
        "textDocument/didOpen",
        json!({"textDocument": {"uri": target_uri, "languageId": "json", "version": 9, "text": "{\"schema_version\":1}\n"}}),
    );
    let reopened_target = harness.barrier(&target_uri);
    assert_publish_batch(&reopened_target, &[(&target_uri, Some(9), true)]);

    harness.notify(
        "textDocument/didClose",
        json!({"textDocument": {"uri": alias_uri}}),
    );
    let closed_alias = harness.barrier(&alias_uri);
    assert_publish_batch(
        &closed_alias,
        &[(&alias_uri, None, true), (&target_uri, Some(9), false)],
    );
    harness.notify(
        "textDocument/didClose",
        json!({"textDocument": {"uri": target_uri}}),
    );
    assert_publish_batch(&harness.barrier(&target_uri), &[(&target_uri, None, true)]);
    harness.finish();
}

#[test]
fn active_schema_alias_owner_is_deterministic_in_both_open_orders() {
    let temp = Builder::new()
        .prefix("recite active schema aliases ")
        .tempdir()
        .unwrap_or_else(|error| panic!("temporary schema directory: {error}"));
    let manifest = temp.path().join("recite.project.toml");
    let schema = temp.path().join("schema.json");
    std::fs::write(
        &manifest,
        "format_version = 1\n[project]\nschema = \"schema.json\"\n",
    )
    .unwrap_or_else(|error| panic!("write manifest: {error}"));
    std::fs::write(&schema, "{\"schema_version\":1}\n")
        .unwrap_or_else(|error| panic!("write schema: {error}"));
    std::fs::create_dir(temp.path().join("sub"))
        .unwrap_or_else(|error| panic!("create schema alias parent: {error}"));
    let canonical = file_uri(&schema);
    let alias_a = format!("{}/./schema.json", file_uri(temp.path()));
    let alias_b = format!("{}/sub/../schema.json", file_uri(temp.path()));
    let mut harness = StdioHarness::start(json!({
        "capabilities": {},
        "rootUri": file_uri(temp.path())
    }));

    // B then A exercises the owner reselection path; the other order is
    // covered by the retirement lifecycle test below.
    harness.notify(
        "textDocument/didOpen",
        json!({"textDocument": {"uri": alias_b, "languageId": "json", "version": 8, "text": "{\"schema_version\":1}\n"}}),
    );
    let opened_b = harness.barrier(&alias_b);
    assert_publish_batch(
        &opened_b,
        &[(&canonical, None, true), (&alias_b, Some(8), true)],
    );

    harness.notify(
        "textDocument/didOpen",
        json!({"textDocument": {"uri": alias_a, "languageId": "json", "version": 7, "text": "{\"schema_version\":\"a\"}\n"}}),
    );
    let opened_a = harness.barrier(&alias_a);
    assert_publish_batch(
        &opened_a,
        &[(&alias_b, None, true), (&alias_a, Some(7), false)],
    );

    harness.notify(
        "textDocument/didChange",
        json!({
            "textDocument": {"uri": alias_b, "version": 9},
            "contentChanges": [{"text": "{\"schema_version\":1}\n"}]
        }),
    );
    assert!(
        harness.barrier(&alias_b).is_empty(),
        "non-owner schema changes must be silent"
    );

    harness.notify(
        "textDocument/didClose",
        json!({"textDocument": {"uri": alias_a}}),
    );
    let closed_a = harness.barrier(&alias_a);
    assert_publish_batch(
        &closed_a,
        &[(&alias_a, None, true), (&alias_b, Some(9), true)],
    );
    harness.finish();
}

#[test]
fn retired_schema_aliases_keep_target_retirement_until_final_close() {
    let temp = Builder::new()
        .prefix("recite retired schema aliases ")
        .tempdir()
        .unwrap_or_else(|error| panic!("temporary schema directory: {error}"));
    let manifest = temp.path().join("recite.project.toml");
    let old_schema = temp.path().join("schema-old.json");
    let new_schema = temp.path().join("schema-new.json");
    std::fs::write(
        &manifest,
        "format_version = 1\n[project]\nschema = \"schema-old.json\"\n",
    )
    .unwrap_or_else(|error| panic!("write manifest: {error}"));
    std::fs::write(&old_schema, "{\"schema_version\":1}\n")
        .unwrap_or_else(|error| panic!("write old schema: {error}"));
    std::fs::write(&new_schema, "{\"schema_version\":\"new\"}\n")
        .unwrap_or_else(|error| panic!("write new schema: {error}"));
    std::fs::create_dir(temp.path().join("sub"))
        .unwrap_or_else(|error| panic!("create schema alias parent: {error}"));

    let schema_a = format!("{}/./schema-old.json", file_uri(temp.path()));
    let schema_b = format!("{}/sub/../schema-old.json", file_uri(temp.path()));
    let new_schema_uri = file_uri(&new_schema);
    let manifest_uri = file_uri(&manifest);
    let mut harness = StdioHarness::start(json!({
        "capabilities": {},
        "rootUri": file_uri(temp.path())
    }));

    for (uri, version) in [(&schema_a, 7_i64), (&schema_b, 8_i64)] {
        harness.notify(
            "textDocument/didOpen",
            json!({
                "textDocument": {
                    "uri": uri,
                    "languageId": "json",
                    "version": version,
                    "text": "{\"schema_version\":\"overlay\"}\n"
                }
            }),
        );
    }
    let opened_a = harness.barrier(&schema_a);
    assert_publish_batch(
        &opened_a,
        &[
            (&file_uri(&old_schema), None, true),
            (&schema_a, Some(7), false),
        ],
    );
    let opened_b = harness.barrier(&schema_b);
    assert!(
        opened_b.is_empty(),
        "second alias should not publish stale diagnostics: {opened_b:?}"
    );

    std::fs::write(
        &manifest,
        "format_version = 1\n[project]\nschema = \"schema-new.json\"\n",
    )
    .unwrap_or_else(|error| panic!("switch schema target: {error}"));
    harness.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [{ "uri": manifest_uri, "type": 2 }] }),
    );
    let switched = harness.barrier(&schema_a);
    assert_eq!(switched.len(), 3, "schema switch batch: {switched:?}");
    assert_publish_batch(
        &switched,
        &[
            (&schema_a, Some(7), true),
            (&schema_b, Some(8), true),
            (&new_schema_uri, None, false),
        ],
    );

    harness.notify(
        "textDocument/didClose",
        json!({ "textDocument": { "uri": schema_a.clone() } }),
    );
    let closed_a = harness.barrier(&schema_a);
    assert_publish_batch(
        &closed_a,
        &[(&schema_a, None, true), (&schema_b, Some(8), true)],
    );

    harness.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": schema_a.clone(),
                "languageId": "json",
                "version": 9,
                "text": "{\"schema_version\":\"reopened\"}\n"
            }
        }),
    );
    let reopened_a = harness.barrier(&schema_a);
    assert_publish_batch(&reopened_a, &[(&schema_a, Some(9), true)]);

    harness.notify(
        "textDocument/didClose",
        json!({ "textDocument": { "uri": schema_b.clone() } }),
    );
    let closed_b = harness.barrier(&schema_b);
    assert_publish_batch(
        &closed_b,
        &[(&schema_b, None, true), (&schema_a, Some(9), false)],
    );

    harness.notify(
        "textDocument/didClose",
        json!({ "textDocument": { "uri": schema_a.clone() } }),
    );
    let closed_final = harness.barrier(&schema_a);
    assert_publish_batch(&closed_final, &[(&schema_a, None, true)]);
    harness.finish();
}

fn assert_publish_batch(messages: &[Value], expected: &[(&str, Option<i64>, bool)]) {
    assert_eq!(
        messages.len(),
        expected.len(),
        "notification batch: {messages:?}"
    );
    for (message, (uri, version, empty)) in messages.iter().zip(expected) {
        assert_eq!(message["method"], "textDocument/publishDiagnostics");
        assert_eq!(message["params"]["uri"], *uri);
        assert_eq!(message["params"]["version"].as_i64(), *version);
        assert_eq!(
            message["params"]["diagnostics"]
                .as_array()
                .is_some_and(Vec::is_empty),
            *empty,
            "diagnostics for {uri}: {message}"
        );
    }
}
