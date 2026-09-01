use serde_json::json;
use tempfile::TempDir;

use crate::workspace::{DiagnosticRefresh, WorkspaceConfig};

use super::super::support::{block_names, file_uri, test_workspace, write_file};

pub(crate) fn manifest_refresh_rekeys_open_overlay() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let nested = temp.path().join("nested");
    write_file(
        temp.path(),
        "recite.project.toml",
        "format_version = 1\n[discovery]\nsource_roots = [\"nested\"]\n",
    );
    write_file(temp.path(), "nested/scene.recite", ":: saved\n");
    let params = serde_json::from_value(json!({
        "rootUri": file_uri(&nested).as_str(),
        "capabilities": {},
    }))
    .unwrap_or_else(|error| panic!("initialize params: {error}"));
    let mut workspace = test_workspace(WorkspaceConfig::from_initialize_params(&params));
    let uri = file_uri(&nested.join("scene.recite"));

    let refresh = workspace.open(uri.clone(), 7, "oops\n:: overlay\n".to_owned());
    assert!(refresh.is_some());
    assert_eq!(workspace.snapshot().summaries().len(), 1);
    assert_eq!(workspace.snapshot().summaries()[0].version, Some(7));
    assert_eq!(
        workspace.snapshot().summaries()[0].project_relative_path(),
        Some("nested/scene.recite")
    );

    write_file(
        temp.path(),
        "nested/recite.project.toml",
        "format_version = 1\n",
    );
    workspace.refresh_watched_uri(&file_uri(&nested.join("recite.project.toml")));
    assert_eq!(workspace.snapshot().summaries().len(), 1);
    assert_eq!(block_names(&workspace), ["overlay"]);
    assert_eq!(workspace.snapshot().summaries()[0].version, Some(7));
    assert_eq!(
        workspace.snapshot().summaries()[0].project_relative_path(),
        Some("scene.recite")
    );
    assert!(!workspace.snapshot().summaries()[0].diagnostics.is_empty());
}

pub(crate) fn watched_creation_rekeys_open_overlay() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let source = temp.path().join("src/new.recite");
    write_file(
        temp.path(),
        "recite.project.toml",
        "format_version = 1\n[discovery]\nsource_roots = [\"src\"]\n",
    );
    std::fs::create_dir(temp.path().join("src")).expect("source root");
    let params = serde_json::from_value(json!({
        "rootUri": file_uri(temp.path()).as_str(),
        "capabilities": {},
    }))
    .unwrap_or_else(|error| panic!("initialize params: {error}"));
    let mut workspace = test_workspace(WorkspaceConfig::from_initialize_params(&params));
    let uri = file_uri(&source);

    let refresh = workspace.open(uri.clone(), 9, "oops\n:: open\n".to_owned());
    assert!(refresh.is_some());
    assert_eq!(workspace.snapshot().summaries().len(), 1);
    assert_eq!(
        workspace.snapshot().summaries()[0].project_relative_path(),
        Some("src/new.recite")
    );

    write_file(temp.path(), "src/new.recite", ":: saved\n");
    let refreshes = workspace.refresh_watched_uri(&uri);

    assert!(refreshes.iter().any(|refresh| matches!(
        refresh,
        DiagnosticRefresh::Publish(diagnostics)
            if diagnostics.uri == uri
                && diagnostics.version == Some(9)
                && diagnostics.text == "oops\n:: open\n"
    )));
    assert_eq!(workspace.snapshot().summaries().len(), 1);
    assert_eq!(block_names(&workspace), ["open"]);
    assert_eq!(workspace.snapshot().summaries()[0].version, Some(9));
    assert_eq!(
        workspace.snapshot().summaries()[0].project_relative_path(),
        Some("src/new.recite")
    );
    assert!(!workspace.snapshot().summaries()[0].diagnostics.is_empty());
}

pub(crate) fn duplicate_open_is_ignored_transactionally() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let source = temp.path().join("scene.recite");
    write_file(temp.path(), "scene.recite", ":: saved\n");
    let mut workspace = test_workspace(WorkspaceConfig::for_roots(vec![temp.path().to_owned()]));
    let uri = file_uri(&source);

    let refresh = workspace.open(uri.clone(), 5, "oops\n:: original\n".to_owned());
    assert!(refresh.is_some());
    let generation = workspace.generation();
    let summary = workspace.snapshot().summaries()[0].clone();

    assert!(
        workspace
            .open(uri, 5, ":: replacement\n".to_owned())
            .is_none()
    );
    assert_eq!(workspace.generation(), generation);
    assert_eq!(workspace.snapshot().generation(), generation);
    assert_eq!(workspace.snapshot().summaries().len(), 1);
    assert_eq!(workspace.snapshot().summaries()[0].version, summary.version);
    assert_eq!(workspace.snapshot().summaries()[0].blocks, summary.blocks);
    assert_eq!(
        workspace.snapshot().summaries()[0].diagnostics,
        summary.diagnostics
    );
}
