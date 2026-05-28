use recite_core::*;

#[test]
fn source_spans_support_points_and_ranges() {
    let start = SourcePosition::new(3, 5).expect("valid source position");
    let end = SourcePosition::new(3, 12).expect("valid source position");

    let point = SourceSpan::point("dialogue/tavern.recite", start);
    assert_eq!(point.file, "dialogue/tavern.recite");
    assert_eq!(point.start, start);
    assert_eq!(point.start.line(), 3);
    assert_eq!(point.start.column(), 5);
    assert_eq!(point.end, None);

    let range = SourceSpan::new("dialogue/tavern.recite", start, Some(end));
    assert_eq!(range.end, Some(end));
}

#[test]
fn source_positions_reject_zero_line_or_column() {
    assert_eq!(
        SourcePosition::new(0, 1),
        Err(CoreValueError::ZeroSourceLine)
    );
    assert_eq!(
        SourcePosition::new(1, 0),
        Err(CoreValueError::ZeroSourceColumn)
    );
}

#[test]
fn diagnostics_keep_stable_structured_fields() {
    let primary = SourceSpan::new(
        "dialogue/tavern.recite",
        SourcePosition::new(8, 1).expect("valid source position"),
        Some(SourcePosition::new(8, 14).expect("valid source position")),
    );
    let related = RelatedSpan::new(
        SourceSpan::point(
            "dialogue/tavern.recite",
            SourcePosition::new(2, 4).expect("valid source position"),
        ),
        "first declaration is here",
    );

    let diagnostic = Diagnostic::new(
        DiagnosticCode::new("RECITE_ID001").expect("valid diagnostic code"),
        DiagnosticSeverity::Error,
        "duplicate line ID",
        primary.clone(),
    )
    .with_related([related.clone()])
    .with_help("rename one of the duplicate IDs");

    assert_eq!(diagnostic.code.as_str(), "RECITE_ID001");
    assert_eq!(diagnostic.code.to_string(), "RECITE_ID001");
    assert_eq!(diagnostic.severity, DiagnosticSeverity::Error);
    assert_eq!(diagnostic.span, primary);
    assert_eq!(diagnostic.related, vec![related]);
    assert_eq!(
        diagnostic.help.as_deref(),
        Some("rename one of the duplicate IDs")
    );
}

#[test]
fn diagnostic_codes_reject_empty_or_non_namespaced_values() {
    assert_eq!(
        DiagnosticCode::new(""),
        Err(CoreValueError::EmptyDiagnosticCode)
    );
    assert_eq!(
        DiagnosticCode::new("ID001"),
        Err(CoreValueError::NonNamespacedDiagnosticCode(
            "ID001".to_owned()
        ))
    );
    assert_eq!(
        DiagnosticCode::new("recite_id001"),
        Err(CoreValueError::NonNamespacedDiagnosticCode(
            "recite_id001".to_owned()
        ))
    );
}

#[test]
fn id_wrappers_are_explicit_and_display_their_inner_value() {
    let line_id = LineId::new("tavern_intro_001").expect("valid line ID");
    let same_line_id = LineId::try_from("tavern_intro_001").expect("valid line ID");
    let choice_id = ChoiceId::new("ask_for_room").expect("valid choice ID");

    assert_eq!(line_id, same_line_id);
    assert_eq!(line_id.as_str(), "tavern_intro_001");
    assert_eq!(line_id.to_string(), "tavern_intro_001");
    assert_eq!(choice_id.as_str(), "ask_for_room");
}

#[test]
fn id_wrappers_reject_empty_values() {
    assert_eq!(
        LineId::new(""),
        Err(CoreValueError::EmptyId { kind: "LineId" })
    );
    assert_eq!(
        ChoiceId::new("  "),
        Err(CoreValueError::EmptyId { kind: "ChoiceId" })
    );
    assert_eq!(
        BlockId::new(""),
        Err(CoreValueError::EmptyId { kind: "BlockId" })
    );
    assert_eq!(
        EffectId::new(""),
        Err(CoreValueError::EmptyId { kind: "EffectId" })
    );
    assert_eq!(
        SpeakerId::new(""),
        Err(CoreValueError::EmptyId { kind: "SpeakerId" })
    );
}

#[test]
fn metadata_preserves_source_order_and_repeated_keys() {
    let mut metadata = Metadata::new();
    metadata.push(MetadataEntry::new("sfx", ScalarValue::from("door")));
    metadata.push(MetadataEntry::new("portrait", ScalarValue::from("neutral")));
    metadata.push(
        MetadataEntry::new("sfx", ScalarValue::from("mug")).with_source_span(SourceSpan::point(
            "dialogue/tavern.recite",
            SourcePosition::new(4, 9).expect("valid source position"),
        )),
    );

    let keys = metadata
        .iter()
        .map(|entry| entry.key.as_str())
        .collect::<Vec<_>>();

    assert_eq!(keys, ["sfx", "portrait", "sfx"]);
    assert_eq!(metadata.len(), 3);
    assert!(!metadata.is_empty());
    assert!(metadata.as_slice()[2].source_span.is_some());
}

#[test]
fn values_support_scalars_and_arrays_of_scalars() {
    let values = [
        Value::from(ScalarValue::from("neutral")),
        Value::from(ScalarValue::from(3_i64)),
        Value::from(ScalarValue::from(1.5_f64)),
        Value::from(ScalarValue::from(true)),
        Value::Array(vec![
            ScalarValue::from("door"),
            ScalarValue::from("mug"),
            ScalarValue::from(false),
        ]),
    ];

    assert_eq!(
        values[0],
        Value::Scalar(ScalarValue::String("neutral".to_owned()))
    );
    assert_eq!(values[1], Value::Scalar(ScalarValue::Integer(3)));
    assert_eq!(values[2], Value::Scalar(ScalarValue::Float(1.5)));
    assert_eq!(values[3], Value::Scalar(ScalarValue::Boolean(true)));
    assert_eq!(
        values[4],
        Value::Array(vec![
            ScalarValue::String("door".to_owned()),
            ScalarValue::String("mug".to_owned()),
            ScalarValue::Boolean(false),
        ])
    );
}
