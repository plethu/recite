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

pub(super) fn typed_clause_and_schema_ranges_exclude_delimiters() {
    let temp = TempDir::new().expect("tempdir");
    write_file(
        temp.path(),
        "schema.json",
        include_str!("../../../../fixtures/schema/valid/generated_manifest.json"),
    );
    let root_uri = file_uri(temp.path());
    let schema_path = temp.path().join("schema.json");
    let mut harness = Harness::start_with_result(serde_json::json!({
        "capabilities": {
            "general": { "positionEncodings": ["utf-16"] }
        },
        "rootUri": root_uri.as_str(),
        "initializationOptions": { "schema": schema_path.display().to_string() }
    }))
    .0;
    let source_uri = file_uri(&temp.path().join("dialogue/ranges.recite"));
    let source = concat!(
        ":: start default speaker=hazel\r\n",
        "? ask@a1b2c3d4e5f60718293a requires=(trust_gte(hazel, rhea, 3)) reason=innkeeper_trust_hint\r\n",
        "  😀 innkeeper_trust_hint, ordinary prose\r\n",
    );
    harness.did_open(source_uri.clone(), 1, source);
    let _ = harness.recv_publish_diagnostics();

    let clause = harness
        .hover(source_uri.clone(), position_inside(source, "requires"))
        .expect("typed requires clause hover");
    assert_eq!(
        clause.range,
        Some(lsp_types::Range::new(
            position_after(source, "? ask@a1b2c3d4e5f60718293a "),
            position_after(
                source,
                "? ask@a1b2c3d4e5f60718293a requires=(trust_gte(hazel, rhea, 3))"
            ),
        )),
        "requires range must stop before the following metadata field",
    );

    let prose = harness
        .hover(source_uri, position_after(source, "  😀 innkeeper_trust"))
        .expect("schema prose hover");
    assert_eq!(
        prose.range,
        Some(lsp_types::Range::new(
            position_after(source, "  😀 "),
            position_after(source, "  😀 innkeeper_trust_hint"),
        )),
        "schema token range must stop before the comma",
    );
    harness.finish();
}

fn range(start_line: u32, start_character: u32, end_line: u32, end_character: u32) -> Range {
    Range {
        start: Position::new(start_line, start_character),
        end: Position::new(end_line, end_character),
    }
}

pub(super) fn condition_marker_completion_and_hover_follow_parser_boundaries() {
    let temp = TempDir::new().expect("tempdir");
    write_file(
        temp.path(),
        "schema.json",
        include_str!("../../../../fixtures/schema/valid/generated_manifest.json"),
    );
    let root_uri = file_uri(temp.path());
    let schema_path = temp.path().join("schema.json");
    let mut harness = Harness::start_with_result(serde_json::json!({
        "capabilities": {
            "general": { "positionEncodings": ["utf-16"] }
        },
        "rootUri": root_uri.as_str(),
        "initializationOptions": { "schema": schema_path.display().to_string() }
    }))
    .0;
    let source_uri = file_uri(&temp.path().join("dialogue/markers.recite"));
    let source = concat!(
        ":: start default\n",
        "\t:if\n",
        "\t:match\n",
        "\t:if\ttrust_\n",
        "\tordinary prose :if trust_gte\n",
    );
    harness.did_open(source_uri.clone(), 1, source);
    let _ = harness.recv_publish_diagnostics();

    assert!(
        completion_labels(
            harness
                .completion(source_uri.clone(), position_after(source, "\t:if"))
                .expect("bare :if completion")
        )
        .contains(&"trust_gte".to_owned())
    );
    assert!(
        completion_labels(
            harness
                .completion(source_uri.clone(), position_after(source, "\t:match"))
                .expect("bare :match completion")
        )
        .contains(&"trust_gte".to_owned())
    );
    assert!(
        completion_labels(
            harness
                .completion(source_uri.clone(), position_after(source, "\t:if\ttrust_"),)
                .expect("tab-separated condition completion")
        )
        .contains(&"trust_gte".to_owned())
    );

    let marker_hover = harness
        .hover(source_uri.clone(), Position::new(3, 2))
        .expect("hover on the exact :if marker");
    assert_eq!(
        marker_hover.range,
        Some(lsp_types::Range::new(
            Position::new(3, 1),
            Position::new(3, 4),
        ))
    );
    assert!(
        harness
            .hover(source_uri.clone(), Position::new(3, 0))
            .is_none(),
        "indentation before a marker must not receive marker hover"
    );
    assert!(
        harness
            .hover(source_uri.clone(), Position::new(3, 4))
            .is_none(),
        "whitespace and arguments after a marker must not receive marker hover"
    );
    assert!(
        harness
            .hover(source_uri, position_after(source, "\tordinary prose "))
            .is_none(),
        "ordinary prose must not receive marker hover"
    );
    harness.finish();
}

fn position_after(source: &str, needle: &str) -> Position {
    let byte_index = source
        .find(needle)
        .expect("range needle")
        .saturating_add(needle.len());
    let mut line = 0_u32;
    let mut character = 0_u32;
    for value in source[..byte_index].chars() {
        if value == '\n' {
            line = line.saturating_add(1);
            character = 0;
        } else {
            character = character.saturating_add(value.len_utf16() as u32);
        }
    }
    Position::new(line, character)
}

fn position_inside(source: &str, needle: &str) -> Position {
    let byte_index = source.find(needle).expect("hover needle").saturating_add(1);
    let mut line = 0_u32;
    let mut character = 0_u32;
    for value in source[..byte_index].chars() {
        if value == '\n' {
            line = line.saturating_add(1);
            character = 0;
        } else {
            character = character.saturating_add(value.len_utf16() as u32);
        }
    }
    Position::new(line, character)
}

fn completion_labels(response: lsp_types::CompletionResponse) -> Vec<String> {
    match response {
        lsp_types::CompletionResponse::Array(items) => {
            items.into_iter().map(|item| item.label).collect()
        }
        lsp_types::CompletionResponse::List(list) => {
            list.items.into_iter().map(|item| item.label).collect()
        }
    }
}
