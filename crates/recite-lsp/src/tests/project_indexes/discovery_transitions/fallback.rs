use serde_json::json;
use tempfile::TempDir;

#[cfg(windows)]
use crate::summary::{FileIdentity, OpenFileIdentity, OpenFileScope};
#[cfg(windows)]
use crate::workspace::document_key_for_identity;
use crate::workspace::{DiagnosticRefresh, WorkspaceConfig};

use super::super::super::support::{block_names, file_uri, test_workspace, write_file};

pub(crate) fn all() {
    malformed_workspace_root_does_not_block_independent_root();
    manifestless_watcher_keeps_builtin_exclusions();
    manifestless_multi_root_documents_keep_project_relative_keys();
    multi_root_documents_keep_project_relative_keys();
    workspace_folders_preserve_manifest_and_fallback_projects();
    nested_fallback_root_overrides_manifest_exclusion_across_refresh();
    excluded_open_files_remain_diagnosable_without_project_membership();
    manifestless_builtin_exclusions_stay_out_of_shared_state();
    sibling_fallback_builtin_exclusions_stay_out_of_shared_state();
    #[cfg(windows)]
    synthetic_windows_paths_keep_drive_identity_in_fallback_keys();
}

pub(crate) fn malformed_workspace_root_does_not_block_independent_root() {
    for malformed_first in [true, false] {
        let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
        let malformed = temp.path().join("malformed");
        let valid = temp.path().join("valid");
        write_file(&malformed, "recite.project.toml", "format_version = [\n");
        write_file(&malformed, "leaked.recite", ":: leaked\n");
        write_file(&valid, "later.recite", ":: later\n");

        let ordered_roots = if malformed_first {
            [&malformed, &valid]
        } else {
            [&valid, &malformed]
        };
        let params = serde_json::from_value(json!({
            "workspaceFolders": ordered_roots
                .iter()
                .map(|root| json!({ "uri": file_uri(root).as_str(), "name": "workspace" }))
                .collect::<Vec<_>>(),
            "capabilities": {},
        }))
        .unwrap_or_else(|error| panic!("initialize params: {error}"));
        let mut workspace = test_workspace(WorkspaceConfig::from_initialize_params(&params));

        assert_eq!(block_names(&workspace), ["later"]);
        assert_eq!(
            workspace.snapshot().summaries()[0].project_relative_path(),
            Some("valid/later.recite")
        );
        let diagnostics = workspace
            .project_diagnostics()
            .expect("malformed workspace manifest diagnostics");
        let DiagnosticRefresh::Publish(diagnostics) = diagnostics else {
            panic!("expected malformed manifest diagnostics");
        };
        assert_eq!(
            diagnostics.uri,
            file_uri(&malformed.join("recite.project.toml"))
        );
        assert_eq!(
            diagnostics.diagnostics[0].code.as_str(),
            "RECITE_PROJECT001"
        );

        let refresh = workspace
            .open(
                file_uri(&valid.join("later.recite")),
                1,
                "oops\n".to_owned(),
            )
            .expect("independent valid workspace should remain authorable");
        let DiagnosticRefresh::Publish(diagnostics) = refresh else {
            panic!("valid workspace should publish authoring diagnostics");
        };
        assert!(!diagnostics.diagnostics.is_empty());
    }
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

pub(crate) fn nested_fallback_root_overrides_manifest_exclusion_across_refresh() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let project = temp.path().join("project");
    write_file(
        &project,
        "recite.project.toml",
        "format_version = 1\n[discovery]\nsource_roots = [\"dialogue\"]\n",
    );
    write_file(&project, "dialogue/project.recite", ":: project\n");
    write_file(&project, "drafts/draft.recite", ":: draft\n");
    let params = serde_json::from_value(json!({
        "workspaceFolders": [
            {"uri": file_uri(&project).as_str(), "name": "project"},
            {"uri": file_uri(&project.join("drafts")).as_str(), "name": "drafts"}
        ],
        "capabilities": {},
    }))
    .unwrap_or_else(|error| panic!("initialize params: {error}"));
    let mut workspace = test_workspace(WorkspaceConfig::from_initialize_params(&params));

    assert_eq!(
        workspace
            .snapshot()
            .summaries()
            .iter()
            .filter_map(|summary| summary.project_relative_path())
            .collect::<Vec<_>>(),
        ["dialogue/project.recite", "drafts/draft.recite"]
    );

    write_file(
        &project,
        "recite.project.toml",
        "format_version = 1\n[discovery]\nsource_roots = [\"dialogue\", \"other\"]\n",
    );
    std::fs::create_dir(project.join("other")).expect("other root");
    workspace.save(file_uri(&project.join("recite.project.toml")));
    assert_eq!(
        workspace
            .snapshot()
            .summaries()
            .iter()
            .filter_map(|summary| summary.project_relative_path())
            .collect::<Vec<_>>(),
        ["dialogue/project.recite", "drafts/draft.recite"]
    );
}

pub(crate) fn manifestless_builtin_exclusions_stay_out_of_shared_state() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "visible.recite", ":: visible default\n");
    let target = temp.path().join("target/ignored.recite");
    let hidden = temp.path().join(".hidden/ignored.recite");
    write_file(temp.path(), "target/ignored.recite", ":: target default\n");
    write_file(temp.path(), ".hidden/ignored.recite", ":: hidden default\n");
    let mut workspace = test_workspace(WorkspaceConfig::for_roots(vec![temp.path().to_owned()]));

    for (path, text) in [
        (&target, ":: target default\n"),
        (&hidden, ":: hidden default\n"),
    ] {
        let refresh = workspace
            .open(file_uri(path), 1, text.to_owned())
            .unwrap_or_else(|| panic!("excluded open should publish diagnostics"));
        let DiagnosticRefresh::Publish(diagnostics) = refresh else {
            panic!("excluded open should publish diagnostics");
        };
        assert!(diagnostics.diagnostics.is_empty());
    }
    assert_eq!(block_names(&workspace), ["visible"]);
}

pub(crate) fn sibling_fallback_builtin_exclusions_stay_out_of_shared_state() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let project = temp.path().join("project");
    let sibling = temp.path().join("sibling");
    write_file(
        &project,
        "recite.project.toml",
        "format_version = 1\n[discovery]\nsource_roots = [\"dialogue\"]\n",
    );
    write_file(&project, "dialogue/project.recite", ":: project\n");
    write_file(&sibling, "visible.recite", ":: visible\n");
    write_file(&sibling, "target/ignored.recite", ":: ignored\n");
    let params = serde_json::from_value(json!({
        "workspaceFolders": [
            {"uri": file_uri(&project).as_str(), "name": "project"},
            {"uri": file_uri(&sibling).as_str(), "name": "sibling"}
        ],
        "capabilities": {},
    }))
    .unwrap_or_else(|error| panic!("initialize params: {error}"));
    let mut workspace = test_workspace(WorkspaceConfig::from_initialize_params(&params));
    let refresh = workspace
        .open(
            file_uri(&sibling.join("target/ignored.recite")),
            1,
            ":: ignored default\n".to_owned(),
        )
        .unwrap_or_else(|| panic!("excluded open should publish diagnostics"));
    let DiagnosticRefresh::Publish(diagnostics) = refresh else {
        panic!("excluded open should publish diagnostics");
    };
    assert!(diagnostics.diagnostics.is_empty());
    assert_eq!(block_names(&workspace), ["project", "visible"]);
}

#[cfg(windows)]
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
