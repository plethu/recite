#![cfg(test)]

use recite_core::{
    DIAGNOSTIC_RECORD_VERSION, Diagnostic, DiagnosticArgumentValue, DiagnosticCode,
    DiagnosticExplanationPresentation, DiagnosticFiniteFloat, DiagnosticPresentation,
    DiagnosticPresentationError, DiagnosticPresentationId, DiagnosticRecord, DiagnosticRecordError,
    DiagnosticRelatedPresentation, DiagnosticSeverity, RelatedSpan, SourcePosition, SourceSpan,
};

fn presentation(id: &str) -> DiagnosticPresentation {
    DiagnosticPresentation::new(
        DiagnosticPresentationId::new(id).expect("test presentation ID is valid"),
    )
}

fn point(line: u32, column: u32) -> SourceSpan {
    SourceSpan::point(
        "dialogue/intro.recite",
        SourcePosition::new(line, column).expect("test source position is valid"),
    )
}

#[test]
fn presentation_ids_and_argument_names_have_stable_shapes() {
    assert!(DiagnosticPresentationId::new("diagnostic-parse-expected").is_ok());
    assert!(DiagnosticPresentationId::new("Diagnostic-Parse-Expected").is_err());
    assert!(DiagnosticPresentationId::new("diagnostic.parse.expected").is_err());
    assert!(DiagnosticPresentationId::new("diagnostic_parse_expected").is_err());
    assert!(DiagnosticPresentationId::new("diagnostic-").is_err());

    let error = presentation("diagnostic-parse-expected")
        .with_argument(
            "Expected",
            DiagnosticArgumentValue::String("line".to_owned()),
        )
        .expect_err("argument names are lower-case identifiers");
    assert!(matches!(
        error,
        DiagnosticPresentationError::InvalidArgumentName(_)
    ));
}

#[test]
fn named_arguments_are_validated_and_serialised_in_key_order() {
    let presentation = presentation("diagnostic-parse-expected")
        .with_argument(
            "actual",
            DiagnosticArgumentValue::String("choice".to_owned()),
        )
        .expect("valid argument")
        .with_argument("expected", DiagnosticArgumentValue::Integer(2))
        .expect("valid argument");

    let names = presentation
        .arguments()
        .keys()
        .map(String::as_str)
        .collect::<Vec<_>>();
    assert_eq!(names, ["actual", "expected"]);
    assert_eq!(
        serde_json::to_string(&presentation).expect("presentation serialises"),
        r#"{"id":"diagnostic-parse-expected","arguments":{"actual":{"type":"string","value":"choice"},"expected":{"type":"integer","value":2}}}"#
    );

    let duplicate = presentation
        .with_argument("actual", DiagnosticArgumentValue::Boolean(true))
        .expect_err("duplicate argument must not silently overwrite");
    assert!(matches!(
        duplicate,
        DiagnosticPresentationError::DuplicateArgument(name) if name == "actual"
    ));

    let duplicate_wire = r#"{"id":"diagnostic-parse-expected","arguments":{"actual":{"type":"string","value":"one"},"actual":{"type":"string","value":"two"}}}"#;
    assert!(serde_json::from_str::<DiagnosticPresentation>(duplicate_wire).is_err());
}

#[test]
fn finite_float_newtype_rejects_non_finite_values_and_wire_numbers() {
    assert_eq!(
        DiagnosticFiniteFloat::new(f64::NAN),
        Err(DiagnosticPresentationError::NonFiniteFloat)
    );
    assert_eq!(
        DiagnosticArgumentValue::try_float(f64::INFINITY),
        Err(DiagnosticPresentationError::NonFiniteFloat)
    );
    assert!(serde_json::from_str::<DiagnosticFiniteFloat>("1e999").is_err());

    let value = DiagnosticArgumentValue::try_float(1.5).expect("finite float");
    assert_eq!(
        serde_json::to_string(&value).expect("finite float serialises"),
        r#"{"type":"float","value":1.5}"#
    );
}

#[test]
fn record_preserves_related_order_and_structured_guidance() {
    let primary = presentation("diagnostic-identifier-duplicate");
    let related_first = DiagnosticRelatedPresentation::new(
        point(2, 1),
        presentation("diagnostic-identifier-first-declaration"),
    );
    let related_second = DiagnosticRelatedPresentation::new(
        point(5, 1),
        presentation("diagnostic-identifier-second-declaration"),
    );
    let explanation = DiagnosticExplanationPresentation::new(presentation(
        "explanation-identifier-duplicate-meaning",
    ))
    .with_common_causes([
        presentation("explanation-identifier-duplicate-cause-same-id"),
        presentation("explanation-identifier-duplicate-cause-copy-paste"),
    ])
    .with_remediation([presentation(
        "explanation-identifier-duplicate-remediation-rename",
    )]);

    let record = DiagnosticRecord::new(
        DiagnosticCode::new_static("RECITE_ID001"),
        DiagnosticSeverity::Error,
        point(5, 8),
        primary.clone(),
    )
    .with_related([related_first.clone(), related_second.clone()])
    .with_help(Some(presentation("diagnostic-identifier-duplicate-help")))
    .with_explanation(Some(explanation.clone()));

    assert_eq!(record.version(), DIAGNOSTIC_RECORD_VERSION);
    assert_eq!(record.presentation, primary);
    assert_eq!(record.related, [related_first, related_second]);
    assert_eq!(record.explanation, Some(explanation));

    let decoded: DiagnosticRecord =
        serde_json::from_str(&serde_json::to_string(&record).expect("record serialises"))
            .expect("record round-trips");
    assert_eq!(decoded, record);
}

#[test]
fn record_has_versioned_golden_wire_shape() {
    let record = DiagnosticRecord::new(
        DiagnosticCode::new_static("RECITE_PARSE001"),
        DiagnosticSeverity::Error,
        point(1, 1),
        presentation("diagnostic-parse-expected"),
    )
    .with_compatibility_message("expected a statement");

    assert_eq!(
        serde_json::to_string(&record).expect("record serialises"),
        r#"{"version":1,"code":"RECITE_PARSE001","severity":"error","span":{"file":"dialogue/intro.recite","start":{"line":1,"column":1},"end":null},"presentation":{"id":"diagnostic-parse-expected","arguments":{}},"related":[],"help":null,"explanation":null,"compatibility_message":"expected a statement"}"#
    );
}

#[test]
fn record_deserialisation_rejects_unsupported_versions_unknown_fields_and_duplicates() {
    let base = r#"{"version":1,"code":"RECITE_PARSE001","severity":"error","span":{"file":"dialogue/intro.recite","start":{"line":1,"column":1},"end":null},"presentation":{"id":"diagnostic-parse-expected","arguments":{}},"related":[],"help":null,"explanation":null,"compatibility_message":null}"#;
    assert!(
        serde_json::from_str::<DiagnosticRecord>(&base.replace("\"version\":1", "\"version\":2"))
            .is_err()
    );
    let unknown_field = format!("{},\"future\":true}}", &base[..base.len() - 1]);
    assert!(serde_json::from_str::<DiagnosticRecord>(&unknown_field).is_err());
    let duplicate_field = base.replace(
        "\"code\":\"RECITE_PARSE001\"",
        "\"code\":\"RECITE_PARSE001\",\"code\":\"RECITE_ID001\"",
    );
    assert!(serde_json::from_str::<DiagnosticRecord>(&duplicate_field).is_err());
}

#[test]
fn record_deserialisation_rejects_unknown_fields_in_nested_wire_values() {
    let cases = [
        (
            "source span",
            r#"{"version":1,"code":"RECITE_PARSE001","severity":"error","span":{"file":"dialogue/intro.recite","start":{"line":1,"column":1},"end":null,"future":true},"presentation":{"id":"diagnostic-parse-expected","arguments":{}},"related":[],"help":null,"explanation":null,"compatibility_message":null}"#,
        ),
        (
            "source position",
            r#"{"version":1,"code":"RECITE_PARSE001","severity":"error","span":{"file":"dialogue/intro.recite","start":{"line":1,"column":1,"future":true},"end":null},"presentation":{"id":"diagnostic-parse-expected","arguments":{}},"related":[],"help":null,"explanation":null,"compatibility_message":null}"#,
        ),
        (
            "argument value",
            r#"{"version":1,"code":"RECITE_PARSE001","severity":"error","span":{"file":"dialogue/intro.recite","start":{"line":1,"column":1},"end":null},"presentation":{"id":"diagnostic-parse-expected","arguments":{"actual":{"type":"string","value":"line","future":true}}},"related":[],"help":null,"explanation":null,"compatibility_message":null}"#,
        ),
        (
            "related presentation",
            r#"{"version":1,"code":"RECITE_PARSE001","severity":"error","span":{"file":"dialogue/intro.recite","start":{"line":1,"column":1},"end":null},"presentation":{"id":"diagnostic-parse-expected","arguments":{}},"related":[{"span":{"file":"dialogue/intro.recite","start":{"line":2,"column":1},"end":null},"presentation":{"id":"diagnostic-related","arguments":{}},"future":true}],"help":null,"explanation":null,"compatibility_message":null}"#,
        ),
        (
            "explanation presentation",
            r#"{"version":1,"code":"RECITE_PARSE001","severity":"error","span":{"file":"dialogue/intro.recite","start":{"line":1,"column":1},"end":null},"presentation":{"id":"diagnostic-parse-expected","arguments":{}},"related":[],"help":null,"explanation":{"meaning":{"id":"explanation-meaning","arguments":{}},"common_causes":[],"remediation":[],"future":true},"compatibility_message":null}"#,
        ),
    ];

    for (label, wire) in cases {
        assert!(
            serde_json::from_str::<DiagnosticRecord>(wire).is_err(),
            "unknown {label} field must be rejected"
        );
    }
}

#[test]
fn diagnostic_bridge_is_fallible_without_discarding_legacy_context() {
    let missing = Diagnostic::error(
        DiagnosticCode::new_static("RECITE_PARSE001"),
        "expected a statement",
        point(1, 1),
    );
    assert_eq!(
        missing.record(),
        Err(DiagnosticRecordError::MissingPresentation)
    );

    let legacy_related = RelatedSpan::new(point(2, 1), "first declaration");
    let incomplete = missing.clone().with_related([legacy_related.clone()]);
    let Err(DiagnosticRecordError::LegacyContext { related, help }) = incomplete.record() else {
        panic!("incomplete legacy context must remain observable");
    };
    assert_eq!(related.as_slice(), std::slice::from_ref(&legacy_related));
    assert_eq!(help, None);

    let mixed = Diagnostic::error(
        DiagnosticCode::new_static("RECITE_PARSE001"),
        "expected a statement",
        point(1, 1),
    )
    .with_presentation(presentation("diagnostic-parse-expected"))
    .with_related([legacy_related.clone()])
    .with_help("use a statement");

    let Err(DiagnosticRecordError::LegacyContext { related, help }) = mixed.record() else {
        panic!("legacy context must not be silently discarded");
    };
    assert_eq!(related, [legacy_related]);
    assert_eq!(help.as_deref(), Some("use a statement"));
}

#[test]
fn diagnostic_bridge_stores_message_as_explicit_fallback() {
    let primary = presentation("diagnostic-parse-expected");
    let diagnostic = Diagnostic::error(
        DiagnosticCode::new_static("RECITE_PARSE001"),
        "expected a statement",
        point(1, 1),
    )
    .with_presentation(primary.clone())
    .with_help_presentation(presentation("diagnostic-parse-expected-help"));

    let record = diagnostic.record().expect("structured presentation");
    assert_eq!(record.presentation, primary);
    assert_eq!(record.compatibility_message(), Some("expected a statement"));
    assert_eq!(
        record.message_or(Some("esperava uma instrução")),
        Some("esperava uma instrução")
    );
    assert_eq!(record.message_or(None), Some("expected a statement"));
}
