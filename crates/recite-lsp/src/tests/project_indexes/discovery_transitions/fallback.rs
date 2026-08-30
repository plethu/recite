use serde_json::json;
use tempfile::TempDir;

use crate::workspace::WorkspaceConfig;

use super::super::super::support::{block_names, file_uri, test_workspace, write_file};

pub(crate) fn all() {
    manifestless_watcher_keeps_builtin_exclusions();
    manifestless_multi_root_documents_keep_project_relative_keys();
    multi_root_documents_keep_project_relative_keys();
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
