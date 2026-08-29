use std::collections::BTreeMap;

use recite_core::{DiagnosticArgumentValue, SchemaLoadReport, load_schema_manifest_str};

fn assert_structured_diagnostic(
    report: &SchemaLoadReport,
    presentation_id: &str,
    arguments: &[(&str, DiagnosticArgumentValue)],
) {
    let diagnostic = report
        .diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic
                .presentation
                .as_ref()
                .is_some_and(|presentation| presentation.id().as_str() == presentation_id)
        })
        .expect("structured diagnostic");
    let presentation = diagnostic
        .presentation
        .as_ref()
        .expect("structured presentation");
    let expected = arguments
        .iter()
        .map(|(name, value)| ((*name).to_owned(), value.clone()))
        .collect::<BTreeMap<_, _>>();
    assert_eq!(presentation.id().as_str(), presentation_id);
    assert_eq!(presentation.arguments(), &expected);
    assert!(diagnostic.related.is_empty());
    assert!(diagnostic.help.is_none());
    diagnostic.record().expect("diagnostic record");
}

fn string(value: &str) -> DiagnosticArgumentValue {
    DiagnosticArgumentValue::String(value.to_owned())
}

#[test]
fn validation_diagnostics_have_exact_structured_contracts_and_records() {
    let domain_references = load_schema_manifest_str(
        "validation-structured.json",
        r#"{
  "schema_version": 1,
  "metadata_domains": {
    "contextual": {
      "kind": "contextual",
      "selector": "field:speaker",
      "values_by_context": { "rhea": ["flat"] },
      "missing_context": { "policy": "fallback", "domain": "contextual" }
    }
  },
  "metadata": {
    "portrait": {
      "targets": ["line"],
      "type": "symbol",
      "domain": "missing"
    }
  }
}"#,
    );
    assert_structured_diagnostic(
        &domain_references,
        "diagnostic-schema-004-contextual-domain-for-flat",
        &[
            ("owner", string("metadata domain 'contextual' fallback")),
            ("domain", string("contextual")),
        ],
    );
    assert_structured_diagnostic(
        &domain_references,
        "diagnostic-schema-004-unknown-metadata-domain",
        &[
            ("owner", string("metadata 'portrait'")),
            ("domain", string("missing")),
        ],
    );

    let type_references = load_schema_manifest_str(
        "validation-structured-types.json",
        include_str!("../../../../../fixtures/schema/invalid/invalid_type_references.json"),
    );
    assert_structured_diagnostic(
        &type_references,
        "diagnostic-schema-004-unknown-enum",
        &[
            ("owner", string("condition 'thread_stage' return type")),
            ("name", string("thread_stage_kind")),
        ],
    );
    assert_structured_diagnostic(
        &type_references,
        "diagnostic-schema-004-unknown-registry",
        &[
            (
                "owner",
                string("condition 'thread_stage' parameter 'thread_id'"),
            ),
            ("name", string("thread")),
        ],
    );

    let duplicate_definition = load_schema_manifest_str(
        "validation-structured-duplicate.json",
        include_str!("../../../../../fixtures/schema/invalid/duplicate_definitions.json"),
    );
    assert_structured_diagnostic(
        &duplicate_definition,
        "diagnostic-schema-003-duplicate-definition",
        &[
            ("kind", string("type")),
            ("name", string("thread_stage_kind")),
        ],
    );

    let empty_value = load_schema_manifest_str(
        "validation-structured-empty.json",
        r#"{
  "schema_version": 1,
  "producer": { "kind": "", "id": "provider" }
}"#,
    );
    assert_structured_diagnostic(
        &empty_value,
        "diagnostic-schema-001-empty-value",
        &[("field", string("manifest producer kind"))],
    );

    let invalid_name = load_schema_manifest_str(
        "validation-structured-name.json",
        r#"{
  "schema_version": 1,
  "types": {
    "bad name": { "kind": "enum", "values": ["ok"] }
  }
}"#,
    );
    assert_structured_diagnostic(
        &invalid_name,
        "diagnostic-schema-001-invalid-name",
        &[("field", string("type name"))],
    );
}
