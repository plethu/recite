mod support;

use std::path::Path;

use serde_json::{Value, json};
use tempfile::Builder;

use support::stdio::{StdioHarness, file_uri};

const SHARED_SOURCE: &str = ":: source default\n> shared@83709c28414d0ce4659c\n  Shared.\n";

#[test]
fn stdio_nested_fallback_folder_survives_manifest_refresh() {
    let temp = Builder::new()
        .prefix("recite % stdio nested ")
        .tempdir()
        .unwrap_or_else(|error| panic!("temporary workspace: {error}"));
    let project = temp.path().join("project");
    let drafts = project.join("drafts");
    write_file(
        &project,
        "recite.project.toml",
        "format_version = 1\n[discovery]\nsource_roots = [\"dialogue\"]\n",
    );
    write_file(&project, "dialogue/project.recite", SHARED_SOURCE);
    write_file(&project, "drafts/draft.recite", ":: draft default\n");
    let folders = workspace_folders(&[&project, &drafts]);
    let mut harness = StdioHarness::start(json!({
        "capabilities": {},
        "workspaceFolders": folders
    }));

    write_file(
        &project,
        "recite.project.toml",
        "format_version = 1\n[discovery]\nsource_roots = [\"dialogue\", \"other\"]\n",
    );
    std::fs::create_dir(project.join("other")).expect("other root");
    harness.notify(
        "textDocument/didSave",
        json!({ "textDocument": { "uri": file_uri(&project.join("recite.project.toml")) } }),
    );

    let draft = project.join("drafts/draft.recite");
    let draft_uri = file_uri(&draft);
    write_file(&project, "drafts/draft.recite", "oops\n:: draft default\n");
    harness.notify(
        "textDocument/didSave",
        json!({ "textDocument": { "uri": draft_uri.clone() } }),
    );
    let messages = harness.barrier(&draft_uri);
    let published = messages
        .iter()
        .find(|message| message["params"]["uri"] == draft_uri)
        .unwrap_or_else(|| panic!("draft diagnostics notification is missing: {messages:?}"));
    assert!(!diagnostics(&published["params"]).is_empty());
    harness.finish();
}

#[test]
fn stdio_manifestless_builtin_exclusions_remain_diagnosable_and_unshared() {
    let temp = Builder::new()
        .prefix("recite % stdio builtin ")
        .tempdir()
        .unwrap_or_else(|error| panic!("temporary workspace: {error}"));
    write_file(temp.path(), "visible.recite", SHARED_SOURCE);
    write_file(temp.path(), "target/ignored.recite", SHARED_SOURCE);
    write_file(temp.path(), ".hidden/ignored.recite", SHARED_SOURCE);
    let mut harness = StdioHarness::start(json!({
        "capabilities": {},
        "workspaceFolders": workspace_folders(&[temp.path()])
    }));

    let target_uri = file_uri(&temp.path().join("target/ignored.recite"));
    open(&mut harness, &target_uri, SHARED_SOURCE, 1);
    assert!(diagnostics(&diagnostics_after(&mut harness, &target_uri)).is_empty());
    harness.notify(
        "textDocument/didChange",
        json!({
            "textDocument": { "uri": target_uri.clone(), "version": 2 },
            "contentChanges": [{ "text": "oops\n" }]
        }),
    );
    assert!(!diagnostics(&diagnostics_after(&mut harness, &target_uri)).is_empty());

    let hidden_uri = file_uri(&temp.path().join(".hidden/ignored.recite"));
    open(&mut harness, &hidden_uri, SHARED_SOURCE, 1);
    assert!(diagnostics(&diagnostics_after(&mut harness, &hidden_uri)).is_empty());
    harness.finish();
}

#[test]
fn stdio_sibling_fallback_builtin_exclusion_does_not_join_manifest_kernel() {
    let temp = Builder::new()
        .prefix("recite % stdio sibling ")
        .tempdir()
        .unwrap_or_else(|error| panic!("temporary workspace: {error}"));
    let project = temp.path().join("project");
    let sibling = temp.path().join("sibling");
    write_file(
        &project,
        "recite.project.toml",
        "format_version = 1\n[discovery]\nsource_roots = [\"dialogue\"]\n",
    );
    write_file(&project, "dialogue/kept.recite", SHARED_SOURCE);
    write_file(&sibling, "visible.recite", SHARED_SOURCE);
    write_file(&sibling, "target/ignored.recite", SHARED_SOURCE);
    let mut harness = StdioHarness::start(json!({
        "capabilities": {},
        "workspaceFolders": workspace_folders(&[&project, &sibling])
    }));

    let ignored_uri = file_uri(&sibling.join("target/ignored.recite"));
    open(&mut harness, &ignored_uri, SHARED_SOURCE, 1);
    assert!(diagnostics(&diagnostics_after(&mut harness, &ignored_uri)).is_empty());
    harness.finish();
}

#[cfg(unix)]
#[test]
fn stdio_symlinked_workspace_root_preserves_hidden_alias_exclusion() {
    use std::os::unix::fs::symlink;

    let temp = Builder::new()
        .prefix("recite % stdio symlink root ")
        .tempdir()
        .unwrap_or_else(|error| panic!("temporary workspace: {error}"));
    let real = temp.path().join("real");
    let link = temp.path().join("link");
    write_file(&real, ".hidden/ignored.recite", SHARED_SOURCE);
    symlink(&real, &link).unwrap_or_else(|error| panic!("workspace root alias: {error}"));
    let hidden_uri = file_uri(&link.join(".hidden/ignored.recite"));
    let mut harness = StdioHarness::start(json!({
        "capabilities": {},
        "workspaceFolders": workspace_folders(&[&link])
    }));

    open(&mut harness, &hidden_uri, SHARED_SOURCE, 1);
    assert!(diagnostics(&diagnostics_after(&mut harness, &hidden_uri)).is_empty());

    std::fs::remove_file(real.join(".hidden/ignored.recite"))
        .unwrap_or_else(|error| panic!("remove hidden source: {error}"));
    harness.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [{ "uri": hidden_uri.clone(), "type": 3 }] }),
    );
    assert!(diagnostics(&diagnostics_after(&mut harness, &hidden_uri)).is_empty());

    write_file(&real, ".hidden/ignored.recite", SHARED_SOURCE);
    harness.notify(
        "workspace/didChangeWatchedFiles",
        json!({ "changes": [{ "uri": hidden_uri.clone(), "type": 1 }] }),
    );
    assert!(diagnostics(&diagnostics_after(&mut harness, &hidden_uri)).is_empty());
    harness.finish();
}

fn workspace_folders(paths: &[&Path]) -> Vec<Value> {
    paths
        .iter()
        .enumerate()
        .map(|(index, path)| json!({ "uri": file_uri(path), "name": format!("folder-{index}") }))
        .collect()
}

fn open(harness: &mut StdioHarness, uri: &str, text: &str, version: i32) {
    harness.notify(
        "textDocument/didOpen",
        json!({
            "textDocument": {
                "uri": uri,
                "languageId": "recite",
                "version": version,
                "text": text
            }
        }),
    );
}

fn diagnostics(params: &Value) -> &[Value] {
    params["diagnostics"]
        .as_array()
        .unwrap_or_else(|| panic!("diagnostics array is missing: {params}"))
}

fn diagnostics_after(harness: &mut StdioHarness, uri: &str) -> Value {
    let messages = harness.barrier(uri);
    let matches = messages
        .iter()
        .filter(|message| message["method"] == "textDocument/publishDiagnostics")
        .filter(|message| message["params"]["uri"] == uri)
        .map(|message| message["params"].clone())
        .collect::<Vec<_>>();
    assert_eq!(
        matches.len(),
        1,
        "expected one diagnostics notification for {uri}, got {messages:?}"
    );
    matches
        .into_iter()
        .next()
        .unwrap_or_else(|| panic!("diagnostics notification disappeared"))
}

fn write_file(root: &Path, relative: &str, text: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .unwrap_or_else(|error| panic!("create source parent: {error}"));
    }
    std::fs::write(path, text).unwrap_or_else(|error| panic!("write source file: {error}"));
}
