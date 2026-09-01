use serde_json::json;
use tempfile::TempDir;

use crate::workspace::{DiagnosticRefresh, WorkspaceConfig};

use super::super::super::support::{
    block_names, file_uri, full_change, test_workspace, write_file,
};

pub(crate) fn all() {
    malformed_manifest_stays_fail_closed_across_file_lifecycle();
    manifestless_refresh_preserves_discovery_candidate();
    nested_discovery_start_survives_manifest_transitions();
    manifest_refresh_clears_removed_saved_diagnostics_only();
}

pub(crate) fn malformed_manifest_stays_fail_closed_across_file_lifecycle() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "recite.project.toml", "format_version = [\n");
    let source = temp.path().join("source.recite");
    write_file(temp.path(), "source.recite", ":: saved\n");
    let manifest_uri = file_uri(&temp.path().join("recite.project.toml"));
    let source_uri = file_uri(&source);
    let mut workspace = test_workspace(WorkspaceConfig::from_initialize_params(
        &serde_json::from_value(json!({
            "rootUri": file_uri(temp.path()).as_str(),
            "capabilities": {},
        }))
        .expect("initialize params"),
    ));

    workspace.open_refreshes(source_uri.clone(), 1, ":: overlay\n".to_owned());
    write_file(temp.path(), "source.recite", ":: changed\n");
    workspace.save(source_uri.clone());
    workspace.refresh_watched_uri(&source_uri);
    workspace.save(manifest_uri);
    assert_eq!(block_names(&workspace), ["overlay"]);
    workspace.close(source_uri);
    assert!(workspace.snapshot().summaries().is_empty());
}

pub(crate) fn manifestless_refresh_preserves_discovery_candidate() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "source.recite", ":: source\n");
    let manifest = temp.path().join("recite.project.toml");
    let manifest_uri = file_uri(&manifest);
    let params = serde_json::from_value(json!({
        "rootUri": file_uri(temp.path()).as_str(),
        "capabilities": {},
    }))
    .unwrap_or_else(|error| panic!("initialize params: {error}"));
    let mut workspace = test_workspace(WorkspaceConfig::from_initialize_params(&params));
    assert_eq!(block_names(&workspace), ["source"]);

    write_file(temp.path(), "recite.project.toml", "format_version = 1\n");
    workspace.refresh_watched_uri(&manifest_uri);
    assert_eq!(block_names(&workspace), ["source"]);

    std::fs::remove_file(&manifest).expect("remove manifest");
    workspace.refresh_watched_uri(&manifest_uri);
    assert_eq!(block_names(&workspace), ["source"]);

    write_file(temp.path(), "recite.project.toml", "format_version = 1\n");
    workspace.refresh_watched_uri(&manifest_uri);
    assert_eq!(block_names(&workspace), ["source"]);
}

pub(crate) fn nested_discovery_start_survives_manifest_transitions() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let nested = temp.path().join("nested");
    write_file(temp.path(), "nested/a.recite", ":: nested\n");
    let parent_manifest = temp.path().join("recite.project.toml");
    write_file(
        temp.path(),
        "recite.project.toml",
        "format_version = 1\n[discovery]\nsource_roots = [\"nested\"]\n",
    );
    let params = serde_json::from_value(json!({
        "rootUri": file_uri(&nested).as_str(),
        "capabilities": {},
    }))
    .unwrap_or_else(|error| panic!("initialize params: {error}"));
    let mut workspace = test_workspace(WorkspaceConfig::from_initialize_params(&params));
    assert_eq!(block_names(&workspace), ["nested"]);

    workspace.refresh_watched_uri(&file_uri(&parent_manifest));
    std::fs::remove_file(&parent_manifest).expect("remove parent manifest");
    workspace.refresh_watched_uri(&file_uri(&parent_manifest));

    let nested_manifest = nested.join("recite.project.toml");
    write_file(
        temp.path(),
        "nested/recite.project.toml",
        "format_version = 1\n",
    );
    workspace.refresh_watched_uri(&file_uri(&nested_manifest));
    let keys = workspace
        .snapshot()
        .summaries()
        .iter()
        .filter_map(|summary| summary.project_relative_path())
        .collect::<Vec<_>>();
    assert_eq!(keys, ["a.recite"]);

    std::fs::remove_file(&nested_manifest).expect("remove nested manifest");
    workspace.refresh_watched_uri(&file_uri(&nested_manifest));
    write_file(
        temp.path(),
        "recite.project.toml",
        "format_version = 1\n[discovery]\nsource_roots = [\"nested\"]\n",
    );
    workspace.refresh_watched_uri(&file_uri(&parent_manifest));
    let keys = workspace
        .snapshot()
        .summaries()
        .iter()
        .filter_map(|summary| summary.project_relative_path())
        .collect::<Vec<_>>();
    assert_eq!(keys, ["nested/a.recite"]);
}

pub(crate) fn manifest_refresh_clears_removed_saved_diagnostics_only() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "recite.project.toml", "format_version = 1\n");
    let closed = temp.path().join("closed.recite");
    let open = temp.path().join("open.recite");
    write_file(temp.path(), "closed.recite", "oops\n");
    write_file(temp.path(), "open.recite", "oops\n");
    let closed_uri = file_uri(&closed);
    let open_uri = file_uri(&open);
    let manifest_uri = file_uri(&temp.path().join("recite.project.toml"));
    let params = serde_json::from_value(json!({
        "rootUri": file_uri(temp.path()).as_str(),
        "capabilities": {},
    }))
    .unwrap_or_else(|error| panic!("initialize params: {error}"));
    let mut workspace = test_workspace(WorkspaceConfig::from_initialize_params(&params));
    workspace.open_refreshes(open_uri.clone(), 1, ":: overlay\n".to_owned());
    write_file(
        temp.path(),
        "recite.project.toml",
        "format_version = 1\n[discovery]\nsource_roots = [\"other\"]\n",
    );
    std::fs::create_dir(temp.path().join("other")).expect("other root");
    let refreshes = workspace.save(manifest_uri);
    assert!(refreshes.iter().any(|refresh| matches!(
        refresh,
        DiagnosticRefresh::Clear { uri, .. } if uri == &closed_uri
    )));
    assert!(!refreshes.iter().any(|refresh| matches!(
        refresh,
        DiagnosticRefresh::Clear { uri, .. } if uri == &open_uri
    )));
    assert!(block_names(&workspace).is_empty());
    let DiagnosticRefresh::Publish(diagnostics) =
        (match workspace.change(open_uri, 2, vec![full_change("oops\n")]) {
            crate::workspace::WorkspaceChangeResult::Accepted(refresh) => refresh,
            other => panic!("excluded open file should remain diagnosable: {other:?}"),
        })
    else {
        panic!("excluded open file should publish standalone diagnostics");
    };
    assert!(!diagnostics.diagnostics.is_empty());
}
