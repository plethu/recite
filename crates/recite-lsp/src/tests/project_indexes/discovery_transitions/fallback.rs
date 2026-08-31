use serde_json::json;
use tempfile::TempDir;

use crate::summary::{FileIdentity, OpenFileIdentity, OpenFileScope};
use crate::workspace::{DiagnosticRefresh, WorkspaceConfig, document_key_for_identity};

use super::super::super::support::{block_names, file_uri, test_workspace, write_file};

pub(crate) fn all() {
    manifestless_watcher_keeps_builtin_exclusions();
    manifestless_multi_root_documents_keep_project_relative_keys();
    multi_root_documents_keep_project_relative_keys();
    workspace_folders_preserve_manifest_and_fallback_projects();
    excluded_open_files_remain_diagnosable_without_project_membership();
    synthetic_windows_paths_keep_drive_identity_in_fallback_keys();
}

pub(crate) fn manifestless_watcher_keeps_builtin_exclusions() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "source.recite", ":: source\n");
    let params = serde_json::from_value(json!({
        "rootUri": file_uri(temp.path()).as_str(),
        "capabilities": {},
    }))
    .unwrap_or_else(|error| panic!("initialize params: {error}"));
    let mut workspace = test_workspace(WorkspaceConfig::from_initialize_params(&params));

    write_file(temp.path(), "target/new.recite", ":: target\n");
    write_file(temp.path(), ".hidden/new.recite", ":: hidden\n");
    workspace.refresh_watched_uri(&file_uri(&temp.path().join("target/new.recite")));
    workspace.refresh_watched_uri(&file_uri(&temp.path().join(".hidden/new.recite")));
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        write_file(temp.path(), "generated/linked.recite", ":: linked\n");
        symlink(
            temp.path().join("generated/linked.recite"),
            temp.path().join("link.recite"),
        )
        .expect("generated alias");
        workspace.refresh_watched_uri(&file_uri(&temp.path().join("link.recite")));

        write_file(temp.path(), "visible.recite", ":: visible\n");
        symlink(
            temp.path().join("visible.recite"),
            temp.path().join(".hidden/link.recite"),
        )
        .expect("hidden alias");
        workspace.refresh_watched_uri(&file_uri(&temp.path().join(".hidden/link.recite")));
    }
    assert_eq!(block_names(&workspace), ["source"]);
}

pub(crate) fn manifestless_multi_root_documents_keep_project_relative_keys() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let first = temp.path().join("first");
    let second = temp.path().join("second");
    write_file(temp.path(), "first/a.recite", ":: first\n");
    write_file(temp.path(), "second/a.recite", ":: second\n");
    let config = WorkspaceConfig::for_roots(vec![first.clone(), second.clone()]);
    let mut workspace = test_workspace(config);

    workspace.open(
        file_uri(&first.join("a.recite")),
        1,
        ":: open first\n".to_owned(),
    );
    workspace.open(
        file_uri(&second.join("a.recite")),
        1,
        ":: open second\n".to_owned(),
    );
    write_file(temp.path(), "first/a.recite", ":: saved first\n");
    write_file(temp.path(), "second/a.recite", ":: saved second\n");
    workspace.save(file_uri(&first.join("a.recite")));
    workspace.save(file_uri(&second.join("a.recite")));

    let keys = workspace
        .snapshot()
        .summaries()
        .iter()
        .filter_map(|summary| summary.project_relative_path())
        .collect::<Vec<_>>();
    assert_eq!(keys, ["first/a.recite", "second/a.recite"]);
}

pub(crate) fn multi_root_documents_keep_project_relative_keys() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(
        temp.path(),
        "recite.project.toml",
        "format_version = 1\n[discovery]\nsource_roots = [\"src\", \"other\"]\n",
    );
    let src = temp.path().join("src/a.recite");
    let other = temp.path().join("other/a.recite");
    write_file(temp.path(), "src/a.recite", ":: src\n");
    write_file(temp.path(), "other/a.recite", ":: other\n");
    let params = serde_json::from_value(json!({
        "rootUri": file_uri(temp.path()).as_str(),
        "capabilities": {},
    }))
    .unwrap_or_else(|error| panic!("initialize params: {error}"));
    let mut workspace = test_workspace(WorkspaceConfig::from_initialize_params(&params));
    workspace.open(file_uri(&src), 1, ":: live src\n".to_owned());
    workspace.open(file_uri(&other), 1, ":: live other\n".to_owned());
    let keys = workspace
        .snapshot()
        .summaries()
        .iter()
        .filter_map(|summary| summary.project_relative_path())
        .collect::<Vec<_>>();
    assert_eq!(keys, ["other/a.recite", "src/a.recite"]);
}

pub(crate) fn workspace_folders_preserve_manifest_and_fallback_projects() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let manifest_root = temp.path().join("project");
    let fallback_root = temp.path().join("standalone");
    write_file(
        &manifest_root,
        "recite.project.toml",
        "format_version = 1\n[discovery]\nsource_roots = [\"dialogue\"]\n",
    );
    write_file(&manifest_root, "dialogue/project.recite", ":: project\n");
    write_file(&fallback_root, "standalone.recite", ":: standalone\n");
    let params = serde_json::from_value(json!({
        "workspaceFolders": [
            {"uri": file_uri(&manifest_root).as_str(), "name": "project"},
            {"uri": file_uri(&fallback_root).as_str(), "name": "standalone"}
        ],
        "capabilities": {},
    }))
    .unwrap_or_else(|error| panic!("initialize params: {error}"));

    let workspace = test_workspace(WorkspaceConfig::from_initialize_params(&params));
    let keys = workspace
        .snapshot()
        .summaries()
        .iter()
        .filter_map(|summary| summary.project_relative_path())
        .collect::<Vec<_>>();
    assert_eq!(
        keys,
        ["dialogue/project.recite", "standalone/standalone.recite"]
    );
}

pub(crate) fn excluded_open_files_remain_diagnosable_without_project_membership() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(
        temp.path(),
        "recite.project.toml",
        "format_version = 1\n[discovery]\nsource_roots = [\"dialogue\"]\n",
    );
    write_file(temp.path(), "dialogue/kept.recite", ":: kept\n");
    let excluded = temp.path().join("generated.recite");
    write_file(temp.path(), "generated.recite", ":: saved\n");
    let params = serde_json::from_value(json!({
        "rootUri": file_uri(temp.path()).as_str(),
        "capabilities": {},
    }))
    .unwrap_or_else(|error| panic!("initialize params: {error}"));
    let mut workspace = test_workspace(WorkspaceConfig::from_initialize_params(&params));

    let refresh = workspace
        .open(file_uri(&excluded), 1, "oops\n".to_owned())
        .unwrap_or_else(|| panic!("opening an excluded file should publish diagnostics"));
    let DiagnosticRefresh::Publish(diagnostics) = refresh else {
        panic!("opening an excluded file should publish diagnostics");
    };
    assert!(!diagnostics.diagnostics.is_empty());
    assert_eq!(
        workspace
            .snapshot()
            .summaries()
            .iter()
            .map(|summary| summary.project_relative_path())
            .collect::<Vec<_>>(),
        [Some("dialogue/kept.recite")]
    );
}

pub(crate) fn synthetic_windows_paths_keep_drive_identity_in_fallback_keys() {
    let canonical = |drive: &str| OpenFileIdentity {
        uri: format!("file:///{drive}:/project/dialogue.recite")
            .parse()
            .unwrap_or_else(|error| panic!("synthetic Windows URI: {error}")),
        saved_path: Some(std::path::PathBuf::from(format!(
            "{drive}:\\project\\dialogue.recite"
        ))),
        project_relative_path: None,
        scope: OpenFileScope::Standalone,
    };
    let c = document_key_for_identity(&FileIdentity::Open(canonical("C")))
        .unwrap_or_else(|| panic!("C: fallback key"));
    let d = document_key_for_identity(&FileIdentity::Open(canonical("D")))
        .unwrap_or_else(|| panic!("D: fallback key"));

    assert_ne!(c, d);
    assert!(c.as_str().starts_with("~lsp/"));
    assert!(d.as_str().starts_with("~lsp/"));
}
