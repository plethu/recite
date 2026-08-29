use recite_core::{DiagnosticCode, DiagnosticPresentationId, contract_for, load_schema_source_str};

#[test]
fn source_contract_family_is_registered() {
    for id in [
        "diagnostic-schema-001-json-parse",
        "diagnostic-schema-001-toml-parse",
        "diagnostic-schema-001-toml-decode",
        "diagnostic-schema-001-source-non-finite",
        "diagnostic-schema-001-read",
    ] {
        assert!(
            contract_for(
                &DiagnosticCode::new_static("RECITE_SCHEMA001"),
                &DiagnosticPresentationId::new_static(id),
            )
            .is_some(),
            "missing source contract {id}"
        );
    }
}

#[test]
fn source_diagnostics_use_typed_presentations_and_are_recordable() {
    let report = load_schema_source_str("schema.toml", "schema_version = 1\n");
    assert_eq!(report.diagnostics.len(), 1);
    let diagnostic = &report.diagnostics[0];
    assert_eq!(
        diagnostic
            .presentation
            .as_ref()
            .expect("source diagnostic presentation")
            .id()
            .as_str(),
        "diagnostic-schema-001-source-producer-required"
    );
    assert!(diagnostic.related.is_empty());
    assert!(diagnostic.help.is_none());
    diagnostic
        .record()
        .expect("source diagnostic should be recordable");
}
