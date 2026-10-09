#![cfg(unix)]

mod support;

use std::path::{Path, PathBuf};

use serde_json::{Value, json};
use tempfile::{Builder, TempDir};

use support::stdio::{StdioHarness, file_uri};

const VALID_SCHEMA: &str = "{\"schema_version\":1}\n";
const MALFORMED_SOURCE: &str = include_str!(concat!(
    env!("CARGO_MANIFEST_DIR"),
    "/../../fixtures/recite/invalid/parser_marker_leading_prose.recite"
));

#[test]
fn unrelated_uri_retirement_does_not_keep_a_closed_target_retired() {
    let mut scenario = RetiredSchemaAliases::start();
    let target_uri = file_uri(&scenario.target);
    let alias_uri = file_uri(&scenario.alias);

    scenario.harness.notify(
        "textDocument/didClose",
        json!({"textDocument": {"uri": target_uri}}),
    );
    assert_publication(
        &scenario.harness.barrier(&target_uri),
        &target_uri,
        None,
        true,
    );

    // This alias has an independent URI retirement after its former schema
    // target was reactivated. It must stay excluded until its own close.
    scenario.harness.did_change(&alias_uri, 4, MALFORMED_SOURCE);
    assert_publication(
        &scenario.harness.barrier(&alias_uri),
        &alias_uri,
        Some(4),
        true,
    );

    // The unrelated retired alias must not retain this target after its final
    // owner closed. A fresh open now receives ordinary dialogue diagnostics.
    scenario.harness.did_open(&target_uri, 5, MALFORMED_SOURCE);
    let reopened = scenario.harness.barrier(&target_uri);
    assert_publication(&reopened, &target_uri, Some(5), false);
    assert_parse_diagnostic(&reopened[0]);
    scenario.harness.finish();
}

#[test]
fn uri_retired_alias_retargeted_before_close_keeps_the_target_retired() {
    let mut scenario = RetiredSchemaAliases::start();
    let target_uri = file_uri(&scenario.target);
    let alias_uri = file_uri(&scenario.alias);
    let late_alias_uri = format!("{}/./retired.json", file_uri(scenario.temp.path()));

    // No watcher/change event has enrolled this URI into the target map yet.
    // didClose resolves the still-open alias before deciding whether the
    // target's retirement can end.
    retarget(&scenario.alias, &scenario.target);
    scenario.harness.notify(
        "textDocument/didClose",
        json!({"textDocument": {"uri": target_uri}}),
    );
    let closed = scenario.harness.barrier(&target_uri);
    assert_eq!(closed.len(), 2, "close publications: {closed:?}");
    assert_publication(&closed[..1], &target_uri, None, true);
    assert_publication(&closed[1..], &alias_uri, Some(3), true);

    scenario
        .harness
        .did_open(&late_alias_uri, 4, MALFORMED_SOURCE);
    assert_publication(
        &scenario.harness.barrier(&late_alias_uri),
        &late_alias_uri,
        Some(4),
        true,
    );
    scenario.harness.notify(
        "textDocument/didClose",
        json!({"textDocument": {"uri": alias_uri}}),
    );
    let closed_alias = scenario.harness.barrier(&alias_uri);
    assert_eq!(
        closed_alias.len(),
        2,
        "alias close publications: {closed_alias:?}"
    );
    assert_publication(&closed_alias[..1], &alias_uri, None, true);
    assert_publication(&closed_alias[1..], &late_alias_uri, Some(4), true);

    scenario.harness.notify(
        "textDocument/didClose",
        json!({"textDocument": {"uri": late_alias_uri}}),
    );
    assert_publication(
        &scenario.harness.barrier(&late_alias_uri),
        &late_alias_uri,
        None,
        true,
    );
    scenario.harness.did_open(&target_uri, 5, MALFORMED_SOURCE);
    let reopened = scenario.harness.barrier(&target_uri);
    assert_publication(&reopened, &target_uri, Some(5), false);
    assert_parse_diagnostic(&reopened[0]);
    scenario.harness.finish();
}

struct RetiredSchemaAliases {
    temp: TempDir,
    harness: StdioHarness,
    target: PathBuf,
    alias: PathBuf,
}

impl RetiredSchemaAliases {
    fn start() -> Self {
        use std::os::unix::fs::symlink;

        let temp = Builder::new()
            .prefix("recite reactivated schema aliases ")
            .tempdir()
            .unwrap_or_else(|error| panic!("temporary schema workspace: {error}"));
        let target = temp.path().join("retired.json");
        let active = temp.path().join("active.json");
        let replacement = temp.path().join("replacement.json");
        let detached = temp.path().join("detached.recite");
        let alias = temp.path().join("z-alias.json");
        for path in [&target, &active, &replacement] {
            std::fs::write(path, VALID_SCHEMA)
                .unwrap_or_else(|error| panic!("write schema: {error}"));
        }
        std::fs::write(&detached, ":: detached default\n")
            .unwrap_or_else(|error| panic!("write detached dialogue: {error}"));
        symlink(&active, &alias).unwrap_or_else(|error| panic!("schema symlink: {error}"));
        write_manifest(temp.path(), "retired.json");
        let mut harness = StdioHarness::start(json!({
            "capabilities": {},
            "rootUri": file_uri(temp.path())
        }));
        let target_uri = file_uri(&target);
        let alias_uri = file_uri(&alias);
        harness.did_open(&target_uri, 1, VALID_SCHEMA);
        assert_publication(&harness.barrier(&target_uri), &target_uri, Some(1), true);
        switch_schema(&mut harness, temp.path(), "active.json");
        harness.did_open(&alias_uri, 2, VALID_SCHEMA);
        let _ = harness.barrier(&alias_uri);
        switch_schema(&mut harness, temp.path(), "replacement.json");

        // Reactivating active.json ends its target retirement. The alias now
        // resolves elsewhere, so only its exact URI remains retired.
        retarget(&alias, &detached);
        switch_schema(&mut harness, temp.path(), "active.json");
        harness.did_change(&alias_uri, 3, MALFORMED_SOURCE);
        assert_publication(&harness.barrier(&alias_uri), &alias_uri, Some(3), true);
        Self {
            temp,
            harness,
            target,
            alias,
        }
    }
}

fn write_manifest(root: &Path, schema: &str) {
    std::fs::write(
        root.join("recite.project.toml"),
        format!("format_version = 1\n[project]\nschema = \"{schema}\"\n"),
    )
    .unwrap_or_else(|error| panic!("write schema manifest: {error}"));
}

fn switch_schema(harness: &mut StdioHarness, root: &Path, schema: &str) {
    write_manifest(root, schema);
    let manifest_uri = file_uri(&root.join("recite.project.toml"));
    harness.notify(
        "workspace/didChangeWatchedFiles",
        json!({"changes": [{"uri": manifest_uri, "type": 2}]}),
    );
    let _ = harness.barrier(&manifest_uri);
}

fn retarget(alias: &Path, target: &Path) {
    std::fs::remove_file(alias).unwrap_or_else(|error| panic!("remove schema alias: {error}"));
    std::os::unix::fs::symlink(target, alias)
        .unwrap_or_else(|error| panic!("retarget schema alias: {error}"));
}

fn assert_publication(messages: &[Value], uri: &str, version: Option<i64>, empty: bool) {
    assert_eq!(messages.len(), 1, "publication batch: {messages:?}");
    let message = &messages[0];
    assert_eq!(message["method"], "textDocument/publishDiagnostics");
    assert_eq!(message["params"]["uri"], uri);
    assert_eq!(message["params"]["version"].as_i64(), version);
    assert_eq!(
        message["params"]["diagnostics"]
            .as_array()
            .is_some_and(Vec::is_empty),
        empty,
        "diagnostics for {uri}: {message}"
    );
}

fn assert_parse_diagnostic(message: &Value) {
    assert!(
        message["params"]["diagnostics"]
            .as_array()
            .is_some_and(
                |diagnostics| diagnostics.iter().any(|diagnostic| diagnostic["code"]
                    .as_str()
                    .is_some_and(|code| code.starts_with("RECITE_PARSE")))
            ),
        "reopened source diagnostics: {message}"
    );
}
