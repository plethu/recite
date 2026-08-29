use serde_json::json;
use tempfile::TempDir;

use crate::workspace::{DiagnosticRefresh, LspWorkspace, WorkspaceConfig};

use super::super::support::{block_names, file_uri, write_file};

pub(crate) fn all() {
    malformed_manifest_stays_fail_closed_across_file_lifecycle();
    manifestless_refresh_preserves_discovery_candidate();
    multi_root_documents_keep_project_relative_keys();
    #[cfg(unix)]
    symlink_alias_replacement_reconciles_canonical_identity();
    manifest_refresh_clears_removed_saved_diagnostics_only();
}

pub(crate) fn malformed_manifest_stays_fail_closed_across_file_lifecycle() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "recite.project.toml", "format_version = [\n");
    let source = temp.path().join("source.recite");
    write_file(temp.path(), "source.recite", ":: saved\n");
    let manifest_uri = file_uri(&temp.path().join("recite.project.toml"));
    let source_uri = file_uri(&source);
    let mut workspace = LspWorkspace::new(WorkspaceConfig::from_initialize_params(
        &serde_json::from_value(json!({
            "rootUri": file_uri(temp.path()).as_str(),
            "capabilities": {},
        }))
        .expect("initialize params"),
    ));

    workspace.open(source_uri.clone(), 1, ":: overlay\n".to_owned());
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
    let mut workspace = LspWorkspace::new(WorkspaceConfig::from_initialize_params(&params));
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
    let mut workspace = LspWorkspace::new(WorkspaceConfig::from_initialize_params(&params));
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

#[cfg(unix)]
pub(crate) fn symlink_alias_replacement_reconciles_canonical_identity() {
    use std::os::unix::fs::symlink;

    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(temp.path(), "recite.project.toml", "format_version = 1\n");
    write_file(temp.path(), "one.recite", ":: one\n");
    write_file(temp.path(), "two.recite", ":: two\n");
    let alias = temp.path().join("alias.recite");
    symlink(temp.path().join("one.recite"), &alias).expect("initial alias");
    let uri = file_uri(&alias);
    let params = serde_json::from_value(json!({
        "rootUri": file_uri(temp.path()).as_str(),
        "capabilities": {},
    }))
    .unwrap_or_else(|error| panic!("initialize params: {error}"));
    let mut workspace = LspWorkspace::new(WorkspaceConfig::from_initialize_params(&params));
    workspace.save(uri.clone());

    std::fs::remove_file(&alias).expect("remove alias");
    symlink(temp.path().join("two.recite"), &alias).expect("replacement alias");
    workspace.save(uri);
    let keys = workspace
        .snapshot()
        .summaries()
        .iter()
        .filter_map(|summary| summary.project_relative_path())
        .collect::<Vec<_>>();
    assert_eq!(keys, ["one.recite", "two.recite"]);
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
    let mut workspace = LspWorkspace::new(WorkspaceConfig::from_initialize_params(&params));
    workspace.open(open_uri.clone(), 1, ":: overlay\n".to_owned());
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
    assert_eq!(block_names(&workspace), ["overlay"]);
}
