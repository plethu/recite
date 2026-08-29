use std::collections::BTreeMap;

use recite_core::{
    DiagnosticArgumentValue, DiagnosticCode, DiagnosticPresentationId, SchemaLoadReport,
    contract_for, load_schema_manifest_str,
};

fn string(value: &str) -> DiagnosticArgumentValue {
    DiagnosticArgumentValue::String(value.to_owned())
}

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
    assert_eq!(
        diagnostic.record().expect("diagnostic record").presentation,
        *presentation
    );
}

#[test]
fn availability_and_domain_contract_families_are_registered() {
    for (code, id) in [
        (
            "RECITE_SCHEMA001",
            "diagnostic-schema-001-availability-template-unterminated",
        ),
        (
            "RECITE_SCHEMA001",
            "diagnostic-schema-001-availability-template-invalid-name",
        ),
        (
            "RECITE_SCHEMA001",
            "diagnostic-schema-001-availability-template-unescaped-closing-brace",
        ),
        (
            "RECITE_SCHEMA004",
            "diagnostic-schema-004-unknown-availability-reason",
        ),
        ("RECITE_SCHEMA001", "diagnostic-schema-001-domain-kind"),
        (
            "RECITE_SCHEMA001",
            "diagnostic-schema-001-domain-kind-field",
        ),
    ] {
        assert!(
            contract_for(
                &DiagnosticCode::new_static(code),
                &DiagnosticPresentationId::new_static(id),
            )
            .is_some(),
            "missing availability/domain contract {id}"
        );
    }
}

#[test]
fn availability_placeholder_syntax_uses_finite_structured_variants() {
    for (template, presentation_id, arguments) in [
        (
            "{",
            "diagnostic-schema-001-availability-template-unterminated",
            vec![("reason", string("locked"))],
        ),
        (
            "{Bad-Name}",
            "diagnostic-schema-001-availability-template-invalid-name",
            vec![("reason", string("locked")), ("name", string("Bad-Name"))],
        ),
        (
            "}",
            "diagnostic-schema-001-availability-template-unescaped-closing-brace",
            vec![("reason", string("locked"))],
        ),
    ] {
        let report = load_schema_manifest_str(
            "structured-availability.json",
            &format!(
                r#"{{
  "schema_version": 1,
  "availability_reasons": {{
    "locked": {{ "template": {template:?}, "params": [] }}
  }}
}}"#
            ),
        );
        assert!(report.schema.is_none());
        assert_structured_diagnostic(&report, presentation_id, &arguments);
    }
}

#[test]
fn domain_and_provenance_diagnostics_are_structured() {
    let domain = load_schema_manifest_str(
        "structured-domain.json",
        r#"{
  "schema_version": 1,
  "metadata_domains": {
    "portraits": {
      "kind": "contextual",
      "selector": "field:unknown",
      "values_by_context": { "rhea": ["flat"] },
      "missing_context": { "policy": "diagnostic" }
    }
  }
}"#,
    );
    assert_structured_diagnostic(
        &domain,
        "diagnostic-schema-001-domain-selector",
        &[
            ("domain", string("portraits")),
            ("selector", string("field:unknown")),
        ],
    );

    let provenance = load_schema_manifest_str(
        "structured-provenance.json",
        r#"{
  "schema_version": 1,
  "metadata_domains": {
    "portraits": {
      "kind": "flat",
      "values": ["flat"],
      "value_origins": {
        "missing": { "kind": "asset", "id": "portraits.toml" }
      }
    }
  }
}"#,
    );
    assert_structured_diagnostic(
        &provenance,
        "diagnostic-schema-001-provenance-unknown-value",
        &[
            ("owner", string("metadata domain 'portraits'")),
            ("key", string("missing")),
        ],
    );
}
