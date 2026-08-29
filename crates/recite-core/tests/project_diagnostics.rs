mod project_test_support;

use project_test_support::{assert_diagnostic, assert_recordable, string};
use recite_core::{
    ProjectManifest, ProjectSchema, SourcePosition, SpeakerDefinition,
    validate_project_manifest_source,
};

fn position(line: u32, column: u32) -> SourcePosition {
    SourcePosition::new(line, column).unwrap_or_else(|error| panic!("valid test position: {error}"))
}

#[test]
fn manifest_producers_cover_missing_participants_and_schema_participants() {
    let report = ProjectManifest::load_str_with_spans(
        "recite.project.toml",
        "[[scenes]]\nid = \"opening\"\nasset = \"dialogue.recitec\"\nblock = \"start\"\nparticipants = [\"unknown\"]\n\n[[scenes]]\nid = \"empty\"\nasset = \"dialogue.recitec\"\nblock = \"start\"\n",
    );
    let source = report
        .source
        .unwrap_or_else(|| panic!("valid project source"));
    let mut schema = ProjectSchema::empty_v1();
    schema
        .speakers
        .insert("hazel".to_owned(), SpeakerDefinition { display_name: None });
    let diagnostics = validate_project_manifest_source(&source, Some(&schema));

    assert_diagnostic(
        &diagnostics[0],
        "RECITE_PROJECT008",
        "diagnostic-project-008",
        &[
            ("scene_id", string("opening")),
            ("participant", string("unknown")),
        ],
    );
    assert_diagnostic(
        &diagnostics[1],
        "RECITE_PROJECT005",
        "diagnostic-project-005",
        &[("scene_id", string("empty"))],
    );
}

#[test]
fn duplicate_scene_diagnostic_keeps_later_primary_and_first_related_span() {
    let source = "[[scenes]]\nid = \"same\"\nasset = \"one.recitec\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n\n[[scenes]]\nid = \"same\"\nasset = \"two.recitec\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n";
    let report = ProjectManifest::load_str_with_spans("recite.project.toml", source);
    let loaded = report
        .source
        .as_ref()
        .unwrap_or_else(|| panic!("source loads"));
    let diagnostics = validate_project_manifest_source(loaded, None);
    let duplicate = diagnostics
        .first()
        .unwrap_or_else(|| panic!("duplicate diagnostic"));

    assert_diagnostic(
        duplicate,
        "RECITE_PROJECT002",
        "diagnostic-project-002",
        &[("scene_id", string("same"))],
    );
    assert_eq!(duplicate.related_presentations.len(), 1);
    assert_eq!(duplicate.related, Vec::new());
    assert_eq!(
        duplicate.related_presentations[0]
            .presentation
            .id()
            .as_str(),
        "diagnostic-project-002-related"
    );
    assert_eq!(
        duplicate.related_presentations[0].span,
        loaded.scene_key_span(0, "id")
    );
    assert_eq!(duplicate.span, loaded.scene_key_span(1, "id"));
}

#[test]
fn malformed_project_source_keeps_parser_span_and_structured_record() {
    let source = "[[scenes]]\nid = \"broken\"\nparticipants = [\n";
    let report = ProjectManifest::load_str_with_spans("recite.project.toml", source);
    assert!(report.source.is_none());
    let diagnostic = report
        .diagnostics
        .first()
        .unwrap_or_else(|| panic!("malformed diagnostic"));
    assert_eq!(diagnostic.code.as_str(), "RECITE_PROJECT001");
    let presentation = diagnostic
        .presentation
        .as_ref()
        .unwrap_or_else(|| panic!("structured malformed-project presentation"));
    assert_eq!(presentation.id().as_str(), "diagnostic-project-001");
    assert_eq!(presentation.arguments().len(), 1);
    assert!(matches!(
        presentation.arguments().get("detail"),
        Some(recite_core::DiagnosticArgumentValue::String(_))
    ));
    assert_recordable(diagnostic);
    assert_eq!(diagnostic.span.start, position(3, 17));
}
