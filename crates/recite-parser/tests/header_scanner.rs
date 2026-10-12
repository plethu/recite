use recite_core::ast::{SourceMetadataScalar, SourceMetadataValue, Statement};
use recite_parser::{metadata_assignments, parse};

#[test]
fn quoted_unicode_fields_preserve_byte_offsets_and_character_spans() {
    let header = r#"> prompt@5e0925cd041f2f6df9e2 caption="火 \"é\"" tags=["日 本", "☕"] count=2"#;
    let assignments = metadata_assignments(header);
    assert_eq!(assignments.len(), 3);
    assert_eq!(assignments[0].key, "caption");
    assert_eq!(assignments[0].value, r#""火 \"é\"""#);
    assert_eq!((assignments[0].value_start, assignments[0].end), (38, 50));
    assert_eq!(assignments[1].key, "tags");
    assert_eq!(assignments[1].value, r#"["日 本", "☕"]"#);
    assert_eq!((assignments[1].value_start, assignments[1].end), (56, 74));
    assert_eq!(assignments[2].key, "count");
    assert_eq!(assignments[2].value, "2");
    assert_eq!((assignments[2].value_start, assignments[2].end), (81, 82));

    let source = format!(":: start default\n{header}\n  Text.\n-> END\n");
    let lowered = parse("header.recite", source).lower_source_file();
    assert!(lowered.diagnostics.is_empty(), "{:?}", lowered.diagnostics);
    let Statement::Line(line) = &lowered.source_file.blocks[0].statements[0] else {
        panic!("expected line");
    };
    let entries = line.metadata.as_slice();
    assert_eq!(
        entries[0].value,
        SourceMetadataValue::Scalar(SourceMetadataScalar::StringLiteral("火 \"é\"".into()))
    );
    for (entry, start, end) in [
        (&entries[0], 31, 47),
        (&entries[1], 49, 65),
        (&entries[2], 67, 73),
    ] {
        let span = entry.source_span.as_ref().expect("metadata field span");
        assert_eq!(span.start.line(), 2);
        assert_eq!(span.start.column(), start);
        assert_eq!(span.end.expect("field end").column(), end);
    }
}

#[test]
fn rejected_nested_arrays_preserve_the_following_unicode_field_span() {
    let source = concat!(
        ":: start default\n",
        "> prompt@5e0925cd041f2f6df9e2 tags=[\"火\", [\"é\"]] count=2\n",
        "  Text.\n",
    );
    let lowered = parse("nested.recite", source).lower_source_file();
    assert_eq!(lowered.diagnostics.len(), 1);
    let diagnostic = &lowered.diagnostics[0];
    assert_eq!(diagnostic.code.as_str(), "RECITE_PARSE008");
    assert_eq!(diagnostic.span.start.line(), 2);
    assert_eq!(diagnostic.span.start.column(), 36);
    let Statement::Line(line) = &lowered.source_file.blocks[0].statements[0] else {
        panic!("expected line");
    };
    assert_eq!(line.metadata.len(), 1);
    let count = &line.metadata.as_slice()[0];
    assert_eq!(count.key.as_str(), "count");
    assert_eq!(
        count.value,
        SourceMetadataValue::Scalar(SourceMetadataScalar::Integer(2))
    );
    assert_eq!(
        count
            .source_span
            .as_ref()
            .expect("count span")
            .start
            .column(),
        49
    );
}

#[test]
fn unterminated_quoted_and_bracketed_fields_keep_recovery_spans() {
    for (field, column) in [("caption=\"火 \\", 39), ("tags=[\"火\"", 36)] {
        let source = format!(":: start default\n> prompt@5e0925cd041f2f6df9e2 {field}\n  Text.\n");
        let lowered = parse("malformed.recite", source).lower_source_file();
        assert_eq!(lowered.diagnostics.len(), 1, "{field}");
        let diagnostic = &lowered.diagnostics[0];
        assert_eq!(diagnostic.code.as_str(), "RECITE_PARSE008");
        assert_eq!(diagnostic.span.start.line(), 2);
        assert_eq!(diagnostic.span.start.column(), column);
        assert!(!lowered.recovery.metadata());
    }
}
