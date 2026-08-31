use serde_json::json;
use tempfile::TempDir;

use crate::workspace::{DiagnosticRefresh, WorkspaceConfig};

use super::super::super::support::{block_names, file_uri, test_workspace, write_file};

pub(crate) fn all() {
    malformed_workspace_root_does_not_block_independent_root();
    two_malformed_roots_publish_independent_diagnostics();
    nested_valid_manifest_overrides_malformed_outer_root();
    sibling_manifest_transitions_preserve_unaffected_root();
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
            .project_diagnostics_all()
            .into_iter()
            .next()
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

fn two_malformed_roots_publish_independent_diagnostics() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let first = temp.path().join("first");
    let second = temp.path().join("second");
    write_file(&first, "recite.project.toml", "format_version = [\n");
    write_file(&second, "recite.project.toml", "format_version = [\n");
    let params = serde_json::from_value(json!({
        "workspaceFolders": [
            {"uri": file_uri(&first), "name": "first"},
            {"uri": file_uri(&second), "name": "second"}
        ],
        "capabilities": {},
    }))
    .unwrap_or_else(|error| panic!("initialize params: {error}"));
    let mut workspace = test_workspace(WorkspaceConfig::from_initialize_params(&params));
    let paths = workspace
        .project_diagnostics_all()
        .into_iter()
        .filter_map(|refresh| match refresh {
            DiagnosticRefresh::Publish(diagnostics) => Some(diagnostics.uri),
            DiagnosticRefresh::Clear { .. } => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(
        paths,
        vec![
            file_uri(&first.join("recite.project.toml")),
            file_uri(&second.join("recite.project.toml"))
        ]
    );
    let first_manifest = first.join("recite.project.toml");
    std::fs::remove_file(&first_manifest).expect("remove first manifest");
    let refreshes = workspace.refresh_watched_uri(&file_uri(&first_manifest));
    assert!(refreshes.iter().any(|refresh| matches!(
        refresh,
        DiagnosticRefresh::Clear { uri, .. } if uri == &file_uri(&first_manifest)
    )));
    let remaining = workspace
        .project_diagnostics_all()
        .into_iter()
        .filter_map(|refresh| match refresh {
            DiagnosticRefresh::Publish(diagnostics) => Some(diagnostics.uri),
            DiagnosticRefresh::Clear { .. } => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(remaining, [file_uri(&second.join("recite.project.toml"))]);
}

fn nested_valid_manifest_overrides_malformed_outer_root() {
    for nested_first in [true, false] {
        let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
        let outer = temp.path().join("outer");
        let nested = outer.join("nested");
        write_file(&outer, "recite.project.toml", "format_version = [\n");
        write_file(
            &nested,
            "recite.project.toml",
            "format_version = 1\n[discovery]\nsource_roots = [\"src\"]\n",
        );
        write_file(&nested, "src/kept.recite", ":: kept\n");
        let ordered_roots = if nested_first {
            vec![&nested, &outer]
        } else {
            vec![&outer, &nested]
        };
        let params = serde_json::from_value(json!({
            "workspaceFolders": ordered_roots
                .iter()
                .map(|root| json!({ "uri": file_uri(root), "name": "workspace" }))
                .collect::<Vec<_>>(),
            "capabilities": {},
        }))
        .unwrap_or_else(|error| panic!("initialize params: {error}"));
        let mut workspace = test_workspace(WorkspaceConfig::from_initialize_params(&params));
        assert_eq!(block_names(&workspace), ["kept"]);
        let diagnostics = workspace
            .project_diagnostics_all()
            .into_iter()
            .filter_map(|refresh| match refresh {
                DiagnosticRefresh::Publish(diagnostics) => Some(diagnostics),
                DiagnosticRefresh::Clear { .. } => None,
            })
            .collect::<Vec<_>>();
        assert_eq!(diagnostics.len(), 1);
        assert_eq!(
            diagnostics[0].uri,
            file_uri(&outer.join("recite.project.toml"))
        );

        let nested_manifest = nested.join("recite.project.toml");
        write_file(&nested, "recite.project.toml", "format_version = [\n");
        workspace.refresh_watched_uri(&file_uri(&nested_manifest));
        assert!(block_names(&workspace).is_empty());
        write_file(
            &nested,
            "recite.project.toml",
            "format_version = 1\n[discovery]\nsource_roots = [\"src\"]\n",
        );
        workspace.refresh_watched_uri(&file_uri(&nested_manifest));
        assert_eq!(block_names(&workspace), ["kept"]);
    }
}

fn sibling_manifest_transitions_preserve_unaffected_root() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let stable = temp.path().join("a-stable");
    let changing = temp.path().join("b-changing");
    write_file(
        &stable,
        "recite.project.toml",
        "format_version = 1\n[project]\nschema = \"schema.json\"\n[discovery]\nsource_roots = [\"src\"]\n",
    );
    write_file(
        &stable,
        "schema.json",
        "{\"schema_version\":1,\"producer\":{\"kind\":\"adapter\",\"id\":\"stable\"}}\n",
    );
    write_file(&stable, "src/stable.recite", ":: stable\n");
    write_file(
        &changing,
        "recite.project.toml",
        "format_version = 1\n[discovery]\nsource_roots = [\"src\"]\n",
    );
    write_file(&changing, "src/changing.recite", ":: changing\n");
    let params = serde_json::from_value(json!({
        "workspaceFolders": [
            {"uri": file_uri(&stable), "name": "stable"},
            {"uri": file_uri(&changing), "name": "changing"}
        ],
        "capabilities": {},
    }))
    .unwrap_or_else(|error| panic!("initialize params: {error}"));
    let mut workspace = test_workspace(WorkspaceConfig::from_initialize_params(&params));
    assert_eq!(block_names(&workspace), ["changing", "stable"]);
    assert!(workspace.schema().summary().is_some());

    let changing_manifest = changing.join("recite.project.toml");
    write_file(&changing, "recite.project.toml", "format_version = [\n");
    workspace.refresh_watched_uri(&file_uri(&changing_manifest));
    assert_eq!(block_names(&workspace), ["stable"]);
    assert!(workspace.schema().summary().is_some());
    let failure_uris = workspace
        .project_diagnostics_all()
        .into_iter()
        .filter_map(|refresh| match refresh {
            DiagnosticRefresh::Publish(diagnostics) => Some(diagnostics.uri),
            DiagnosticRefresh::Clear { .. } => None,
        })
        .collect::<Vec<_>>();
    assert_eq!(failure_uris, [file_uri(&changing_manifest)]);

    std::fs::remove_file(&changing_manifest).expect("remove changing manifest");
    workspace.refresh_watched_uri(&file_uri(&changing_manifest));
    assert_eq!(block_names(&workspace), ["changing", "stable"]);
    assert!(workspace.schema().summary().is_some());

    write_file(
        &changing,
        "recite.project.toml",
        "format_version = 1\n[discovery]\nsource_roots = [\"src\"]\n",
    );
    workspace.refresh_watched_uri(&file_uri(&changing_manifest));
    assert_eq!(block_names(&workspace), ["changing", "stable"]);
    assert!(workspace.project_diagnostics_all().is_empty());
}
