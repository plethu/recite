use lsp_types::{GotoDefinitionResponse, Location, Position, Uri};
use tempfile::TempDir;

use crate::workspace::WorkspaceConfig;

use super::super::support::{file_uri, test_workspace, write_file};

pub(crate) fn all() {
    valid_manifest_open_identity_survives_creation_and_alias_owner_switch();
    manifestless_drafts_keep_cross_file_identity_per_root();
}

fn valid_manifest_open_identity_survives_creation_and_alias_owner_switch() {
    #[cfg(unix)]
    {
        use std::os::unix::fs::symlink;

        let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
        let source_root = temp.path().join("src");
        write_file(
            temp.path(),
            "recite.project.toml",
            "format_version = 1\n[discovery]\nsource_roots = [\"src\"]\n",
        );
        write_file(
            temp.path(),
            "src/definitions.recite",
            ":: target\n> target_line@22222222222222222222\n  Target.\n",
        );
        let main = source_root.join("main.recite");
        let main_uri = file_uri(&main);
        let definitions_uri = file_uri(&source_root.join("definitions.recite"));
        let source = concat!(
            ":: start default\n",
            "> intro@11111111111111111111\n",
            "  Hello.\n",
            "-> src/definitions.recite::target\n",
        );
        let mut workspace = test_workspace(WorkspaceConfig::from_initialize_params(
            &serde_json::from_value(serde_json::json!({
                "rootUri": file_uri(temp.path()).as_str(),
                "capabilities": {},
            }))
            .unwrap_or_else(|error| panic!("initialize params: {error}")),
        ));

        workspace.open(main_uri.clone(), 1, source.to_owned());
        assert_eq!(project_key(&workspace, &main_uri), Some("src/main.recite"));
        assert_navigation(
            &mut workspace,
            &main_uri,
            &definitions_uri,
            27,
            "missing draft",
        );

        write_file(temp.path(), "src/main.recite", source);
        workspace.save(main_uri.clone());
        assert_eq!(project_key(&workspace, &main_uri), Some("src/main.recite"));
        assert_navigation(
            &mut workspace,
            &main_uri,
            &definitions_uri,
            27,
            "after didSave",
        );

        symlink(&source_root, temp.path().join("src-alias")).expect("source alias");
        let alias_uri = file_uri(&temp.path().join("src-alias/main.recite"));
        workspace.open(alias_uri.clone(), 9, source.to_owned());
        assert_eq!(project_key(&workspace, &alias_uri), Some("src/main.recite"));
        assert_navigation(
            &mut workspace,
            &alias_uri,
            &definitions_uri,
            27,
            "alias owner switch",
        );
    }
}

fn manifestless_drafts_keep_cross_file_identity_per_root() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    let first = temp.path().join("first");
    let second = temp.path().join("second");
    write_file(
        &first,
        "definitions.recite",
        ":: target\n> target_line@22222222222222222222\n  Target.\n",
    );
    write_file(
        &second,
        "definitions.recite",
        ":: target\n> target_line@22222222222222222222\n  Target.\n",
    );
    let first_main = first.join("main.recite");
    let second_main = second.join("main.recite");
    let first_uri = file_uri(&first_main);
    let second_uri = file_uri(&second_main);
    let first_definitions = file_uri(&first.join("definitions.recite"));
    let second_definitions = file_uri(&second.join("definitions.recite"));
    let first_source = concat!(
        ":: start default\n",
        "> intro@11111111111111111111\n",
        "  Hello.\n",
        "-> first/definitions.recite::target\n",
    );
    let second_source = concat!(
        ":: start default\n",
        "> intro@11111111111111111111\n",
        "  Hello.\n",
        "-> second/definitions.recite::target\n",
    );
    let mut workspace = test_workspace(WorkspaceConfig::for_roots(vec![
        first.clone(),
        second.clone(),
    ]));

    workspace.open(first_uri.clone(), 1, first_source.to_owned());
    workspace.open(second_uri.clone(), 1, second_source.to_owned());
    assert_eq!(
        project_key(&workspace, &first_uri),
        Some("first/main.recite")
    );
    assert_eq!(
        project_key(&workspace, &second_uri),
        Some("second/main.recite")
    );
    assert_clean_snapshot(&workspace);
    assert_navigation(
        &mut workspace,
        &first_uri,
        &first_definitions,
        29,
        "first manifestless root",
    );
    assert_navigation(
        &mut workspace,
        &second_uri,
        &second_definitions,
        30,
        "second manifestless root",
    );

    write_file(&first, "main.recite", first_source);
    workspace.save(first_uri.clone());
    assert_eq!(
        project_key(&workspace, &first_uri),
        Some("first/main.recite")
    );
    assert_eq!(
        project_key(&workspace, &second_uri),
        Some("second/main.recite")
    );
    assert_clean_snapshot(&workspace);
}

fn assert_navigation(
    workspace: &mut crate::workspace::LspWorkspace,
    source: &Uri,
    target: &Uri,
    reference_character: u32,
    phase: &str,
) {
    let Some(GotoDefinitionResponse::Scalar(definition)) =
        workspace.definition(source, Position::new(3, reference_character))
    else {
        panic!("{phase}: cross-file definition should resolve");
    };
    assert_eq!(definition.uri, *target, "{phase}: definition target");

    let references = workspace
        .references(target, Position::new(0, 4), true)
        .unwrap_or_else(|| panic!("{phase}: cross-file references should resolve"));
    assert!(
        references.iter().any(|location| location
            == &Location {
                uri: target.clone(),
                range: lsp_types::Range::new(Position::new(0, 3), Position::new(0, 9)),
            }),
        "{phase}: declaration should be included: {references:?}"
    );
    assert!(
        references.iter().any(|location| location.uri == *source),
        "{phase}: source reference should be included: {references:?}"
    );
}

fn assert_clean_snapshot(workspace: &crate::workspace::LspWorkspace) {
    assert!(
        workspace
            .snapshot()
            .summaries()
            .iter()
            .all(|summary| { summary.diagnostics.is_empty() })
    );
}

fn project_key<'a>(workspace: &'a crate::workspace::LspWorkspace, uri: &Uri) -> Option<&'a str> {
    workspace
        .snapshot()
        .summaries()
        .iter()
        .find(|summary| summary.uri() == uri)
        .and_then(|summary| summary.project_relative_path())
}
