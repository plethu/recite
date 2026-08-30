use lsp_types::{DocumentChanges, OneOf, Position, Range, TextEdit};
use tempfile::TempDir;

use super::support::{Harness, file_uri, harness_for_root, uri, write_file};

pub(super) fn rename_rejects_local_and_qualified_block_collisions() {
    let mut local = Harness::start();
    let local_uri = uri("file:///workspace/dialogue/local-collision.recite");
    local.did_open(
        local_uri.clone(),
        1,
        concat!(
            ":: start default\n",
            "-> target\n",
            ":: target\n",
            ":: renamed\n",
        ),
    );
    let _ = local.recv_publish_diagnostics();
    assert!(
        local
            .rename(local_uri.clone(), Position::new(1, 5), "renamed")
            .is_none(),
        "a local destination block collision must abort rename",
    );
    local.finish();

    let temp = TempDir::new().expect("tempdir");
    write_file(
        temp.path(),
        "main.recite",
        ":: start default\n-> defs.recite::target\n",
    );
    write_file(temp.path(), "defs.recite", ":: target\n:: renamed\n");
    let mut qualified = harness_for_root(temp.path());
    let main_uri = file_uri(&temp.path().join("main.recite"));
    assert!(
        qualified
            .rename(main_uri, Position::new(1, 20), "renamed")
            .is_none(),
        "a qualified destination block collision must abort rename",
    );
    qualified.finish();
}

pub(super) fn rename_projects_cross_file_versions_and_order() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("tempdir: {error}"));
    write_file(
        temp.path(),
        "a.recite",
        ":: start default\n-> b.recite::target\n",
    );
    write_file(temp.path(), "b.recite", ":: target\n");
    let source_uri = file_uri(&temp.path().join("a.recite"));
    let target_uri = file_uri(&temp.path().join("b.recite"));
    let mut harness = harness_for_root(temp.path());
    harness.did_open(
        source_uri.clone(),
        7,
        ":: start default\n-> b.recite::target\n",
    );
    let _ = harness.recv_publish_diagnostics();

    let edit = harness
        .rename(source_uri, Position::new(1, 14), "renamed")
        .expect("cross-file rename response");
    let Some(DocumentChanges::Edits(changes)) = edit.document_changes else {
        panic!("expected versioned document changes");
    };
    assert_eq!(changes.len(), 2);
    assert_eq!(
        changes[0].text_document.uri,
        file_uri(&temp.path().join("a.recite"))
    );
    assert_eq!(changes[0].text_document.version, Some(7));
    assert_eq!(changes[1].text_document.uri, target_uri);
    assert_eq!(changes[1].text_document.version, None);
    assert_eq!(
        changes
            .iter()
            .flat_map(|change| change.edits.iter())
            .map(|edit| match edit {
                OneOf::Left(edit) => edit.clone(),
                OneOf::Right(_) => panic!("expected plain text edit"),
            })
            .collect::<Vec<_>>(),
        [
            TextEdit {
                range: range(1, 13, 1, 19),
                new_text: "renamed".to_owned(),
            },
            TextEdit {
                range: range(0, 3, 0, 9),
                new_text: "renamed".to_owned(),
            },
        ]
    );

    harness.finish();
}

pub(super) fn references_require_unique_navigation() {
    let mut unresolved = Harness::start();
    let unresolved_uri = uri("file:///workspace/dialogue/unresolved-references.recite");
    unresolved.did_open(
        unresolved_uri.clone(),
        1,
        concat!(":: start default\n", "-> missing\n"),
    );
    let _ = unresolved.recv_publish_diagnostics();
    assert!(
        unresolved
            .references(unresolved_uri, Position::new(1, 5), true)
            .is_none(),
        "unresolved navigation must preserve the previous no-result response",
    );
    unresolved.finish();

    let mut partial = Harness::start();
    let partial_uri = uri("file:///workspace/dialogue/partial-references.recite");
    partial.did_open(
        partial_uri.clone(),
        1,
        concat!(":: start default\n", "-> target\n", "->\n", ":: target\n",),
    );
    let _ = partial.recv_publish_diagnostics();
    assert!(
        partial
            .references(partial_uri.clone(), Position::new(1, 5), true)
            .is_none(),
        "partial reference coverage must not produce a subset",
    );
    assert!(
        partial
            .prepare_rename(partial_uri.clone(), Position::new(1, 5))
            .is_none(),
        "prepare rename must not advertise an incomplete edit set",
    );
    assert!(
        partial
            .rename(partial_uri, Position::new(1, 5), "renamed")
            .is_none(),
        "rename must reject partial reference coverage",
    );
    partial.finish();

    let mut ambiguous = Harness::start();
    let ambiguous_uri = uri("file:///workspace/dialogue/ambiguous-references.recite");
    ambiguous.did_open(
        ambiguous_uri.clone(),
        1,
        concat!(
            ":: start default\n",
            "-> target\n",
            ":: target\n",
            ":: target\n",
        ),
    );
    let _ = ambiguous.recv_publish_diagnostics();
    assert!(
        ambiguous
            .references(ambiguous_uri, Position::new(1, 5), true)
            .is_none(),
        "ambiguous navigation must preserve the previous no-result response",
    );
    ambiguous.finish();
}

fn range(start_line: u32, start_character: u32, end_line: u32, end_character: u32) -> Range {
    Range {
        start: Position::new(start_line, start_character),
        end: Position::new(end_line, end_character),
    }
}
