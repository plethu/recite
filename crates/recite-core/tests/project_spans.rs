use recite_core::{ProjectManifest, SourcePosition, project_scene_key_span};

fn position(line: u32, column: u32) -> SourcePosition {
    SourcePosition::new(line, column).unwrap_or_else(|error| panic!("valid test position: {error}"))
}

#[test]
fn source_backed_project_spans_follow_decoded_scene_paths() {
    let source = "# id_extra = \"wrong\"\n[[scenes]] # first\nparticipants = [\"hazel\"]\n# id_extra = \"comment\"\nid = \"opening-λ\"\nasset = \"dialogue.recitec\"\nblock = \"start\"\n\n[[scenes]]\n\"id\" = \"second\"\nparticipants = [\"rhea\"]\nasset = \"second.recitec\"\nblock = \"start\"\n";
    let report = ProjectManifest::load_str_with_spans("recite.project.toml", source);
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    let loaded = report.source.as_ref().expect("source loads");

    let first_id = loaded.scene_key_span(0, "id");
    assert_eq!(first_id.start, position(5, 1));
    assert_eq!(first_id.end, Some(position(5, 3)));

    let second_asset = loaded.scene_key_span(1, "asset");
    assert_eq!(second_asset.start, position(12, 1));
    assert_eq!(second_asset.end, Some(position(12, 6)));

    let missing = loaded.scene_key_span(0, "cinematic_scene");
    assert_eq!(missing.start, position(2, 1));
    assert_eq!(missing.end, Some(position(2, 11)));
}

#[test]
fn malformed_public_scene_span_uses_document_start_recovery_boundary() {
    let source = "[[scenes]]\nid = \"broken\"\nparticipants = [\n";
    let span = project_scene_key_span("recite.project.toml", source, 0, "id");

    // A malformed document has no decoded path or table range. The public
    // compatibility wrapper intentionally recovers at the document start.
    assert_eq!(span.start, position(1, 1));
    assert_eq!(span.end, None);
}
