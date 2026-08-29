use recite_core::{
    DiagnosticArgumentType, DiagnosticArgumentValue, DiagnosticCode, DiagnosticPresentationId,
    contract_for, load_schema_manifest_str, migrated_diagnostic_presentation_contracts,
};
use std::collections::BTreeMap;

mod fingerprint;
mod parameter;

#[test]
fn manifest_contract_family_is_registered() {
    let schema = migrated_diagnostic_presentation_contracts()
        .filter(|contract| contract.code().as_str().starts_with("RECITE_SCHEMA"))
        .collect::<Vec<_>>();
    assert_eq!(schema.len(), 119);
    assert_eq!(
        schema
            .iter()
            .map(|contract| contract.presentation_id())
            .collect::<std::collections::BTreeSet<_>>()
            .len(),
        schema.len()
    );
    for id in [
        "diagnostic-schema-002-unsupported-version",
        "diagnostic-schema-001-schema-version-type",
        "diagnostic-schema-001-float-not-representable",
        "diagnostic-schema-001-producer-export-version",
        "diagnostic-schema-003-duplicate-definition",
        "diagnostic-schema-001-invalid-name",
        "diagnostic-schema-004-invalid-metadata-type",
        "diagnostic-schema-001-producer-content-fingerprint-empty-algorithm",
        "diagnostic-schema-001-producer-content-fingerprint-blake3-hex-shape",
        "diagnostic-schema-001-producer-content-fingerprint-blake3-hex-data",
        "diagnostic-schema-001-producer-content-fingerprint-empty-digest",
        "diagnostic-schema-001-producer-content-fingerprint-blake3-digest-length",
    ] {
        assert!(
            contract_for(
                &DiagnosticCode::new_static(if id.contains("unsupported-version") {
                    "RECITE_SCHEMA002"
                } else if id.contains("duplicate-definition") {
                    "RECITE_SCHEMA003"
                } else if id.contains("invalid-metadata-type") {
                    "RECITE_SCHEMA004"
                } else {
                    "RECITE_SCHEMA001"
                }),
                &DiagnosticPresentationId::new_static(id),
            )
            .is_some(),
            "missing manifest contract {id}"
        );
    }

    let unsupported_version = contract_for(
        &DiagnosticCode::new_static("RECITE_SCHEMA002"),
        &DiagnosticPresentationId::new_static("diagnostic-schema-002-unsupported-version"),
    )
    .expect("unsupported-version contract");
    assert_eq!(
        unsupported_version
            .arguments()
            .iter()
            .map(|argument| (argument.name(), argument.argument_type()))
            .collect::<Vec<_>>(),
        [("version", DiagnosticArgumentType::String)]
    );

    let digest_length = contract_for(
        &DiagnosticCode::new_static("RECITE_SCHEMA001"),
        &DiagnosticPresentationId::new_static(
            "diagnostic-schema-001-producer-content-fingerprint-blake3-digest-length",
        ),
    )
    .expect("digest-length contract");
    assert_eq!(
        digest_length
            .arguments()
            .iter()
            .map(|argument| (argument.name(), argument.argument_type()))
            .collect::<Vec<_>>(),
        [("actual", DiagnosticArgumentType::Integer)]
    );
}

#[test]
fn manifest_basic_lowering_diagnostics_have_exact_presentations() {
    let report = load_schema_manifest_str(
        "basic.json",
        r#"{
  "schema_version": 2,
  "types": {
    "state": { "kind": "struct", "values": ["fresh"] },
    "other": { "kind": "enum", "values": ["fresh", "fresh"] }
  },
  "conditions": {
    "check": { "returns": "int" }
  },
  "effects": {
    "play": { "modes": ["immediate", "immediate", "instant"] }
  },
  "metadata": {
    "tag": {
      "targets": ["sideways", "line"],
      "type": "array:string",
      "domain": "all"
    }
  }
}"#,
    );
    assert!(report.schema.is_none());
    crate::assert_recordable_diagnostics(&report);
    assert_eq!(
        report
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic
                .presentation
                .as_ref()
                .expect("presentation")
                .id()
                .as_str())
            .collect::<Vec<_>>(),
        [
            "diagnostic-schema-002-unsupported-version",
            "diagnostic-schema-001-type-kind",
            "diagnostic-schema-003-value",
            "diagnostic-schema-004-invalid-condition-return",
            "diagnostic-schema-003-effect-mode",
            "diagnostic-schema-001-effect-mode",
            "diagnostic-schema-001-metadata-target",
            "diagnostic-schema-001-metadata-array-type",
            "diagnostic-schema-001-metadata-domain-type",
        ]
    );
    let first = &report.diagnostics[0];
    assert_eq!(
        (
            first.span.file.as_str(),
            first.span.start.line(),
            first.span.start.column(),
            first
                .span
                .end
                .map(|position| (position.line(), position.column()))
        ),
        ("basic.json", 2, 3, Some((2, 19)))
    );
    for (id, arguments) in [
        (
            "diagnostic-schema-002-unsupported-version",
            vec![("version", string("2"))],
        ),
        (
            "diagnostic-schema-001-type-kind",
            vec![("type", string("state")), ("kind", string("struct"))],
        ),
        (
            "diagnostic-schema-003-value",
            vec![
                ("owner", string("enum 'other'")),
                ("value", string("fresh")),
            ],
        ),
        (
            "diagnostic-schema-004-invalid-condition-return",
            vec![
                ("condition", string("check")),
                ("return_type", string("int")),
            ],
        ),
        (
            "diagnostic-schema-003-effect-mode",
            vec![("effect", string("play")), ("mode", string("immediate"))],
        ),
        (
            "diagnostic-schema-001-effect-mode",
            vec![("effect", string("play")), ("mode", string("instant"))],
        ),
        (
            "diagnostic-schema-001-metadata-target",
            vec![("metadata", string("tag")), ("target", string("sideways"))],
        ),
        (
            "diagnostic-schema-001-metadata-array-type",
            vec![
                ("metadata", string("tag")),
                ("type_ref", string("array:string")),
            ],
        ),
        (
            "diagnostic-schema-001-metadata-domain-type",
            vec![
                ("metadata", string("tag")),
                ("type_ref", string("array:string")),
            ],
        ),
    ] {
        assert_structured(&report, id, &arguments);
    }
}

fn assert_structured(
    report: &recite_core::SchemaLoadReport,
    presentation_id: &str,
    arguments: &[(&str, DiagnosticArgumentValue)],
) {
    let diagnostic = report
        .diagnostics
        .iter()
        .find(|d| {
            d.presentation
                .as_ref()
                .is_some_and(|p| p.id().as_str() == presentation_id)
        })
        .unwrap_or_else(|| panic!("missing structured diagnostic {presentation_id}"));
    assert_structured_diagnostic(diagnostic, presentation_id, arguments);
}

fn assert_structured_diagnostic(
    diagnostic: &recite_core::Diagnostic,
    presentation_id: &str,
    arguments: &[(&str, DiagnosticArgumentValue)],
) {
    assert_eq!(
        diagnostic
            .presentation
            .as_ref()
            .expect("structured presentation")
            .id()
            .as_str(),
        presentation_id
    );
    assert!(diagnostic.related.is_empty());
    assert!(diagnostic.help.is_none());
    let expected = arguments
        .iter()
        .map(|(name, value)| ((*name).to_owned(), value.clone()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        diagnostic
            .presentation
            .as_ref()
            .expect("structured presentation")
            .arguments(),
        &expected
    );
    diagnostic
        .record()
        .expect("recordable structured diagnostic");
}

fn string(value: &str) -> DiagnosticArgumentValue {
    DiagnosticArgumentValue::String(value.to_owned())
}
