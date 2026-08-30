use lsp_types::Position;
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
