use recite_core::{
    AvailabilityReasonArgBinding, SchemaLiteralValue, canonical_schema_fingerprint,
    load_schema_manifest_str,
};

fn schema_for(number: &str) -> recite_core::ProjectSchema {
    let source = format!(
        r#"{{
  "schema_version": 1,
  "conditions": {{ "ready": {{ "returns": "bool", "availability_reason": {{
    "reason": "reason", "args": {{ "value": {number} }}
  }} }} }},
  "availability_reasons": {{ "reason": {{
    "template": "{{value}}", "params": [{{ "name": "value", "type": "float" }}]
  }} }}
}}"#
    );
    load_schema_manifest_str("numeric.json", &source)
        .schema
        .unwrap_or_else(|| panic!("numeric manifest should load: {number}"))
}

fn projection_schema_for(number: &str) -> recite_core::ProjectSchema {
    projection_report_for(number)
        .schema
        .unwrap_or_else(|| panic!("projection manifest should load: {number}"))
}

fn projection_report_for(number: &str) -> recite_core::SchemaLoadReport {
    let source = format!(
        r#"{{
  "schema_version": 1,
  "presentation_projectors": {{
    "projector": {{
      "candidates": {{ "kind": "runtime_event", "event": "dialogue" }},
      "outputs": {{
        "badge": {{
          "target": "candidate",
          "kind": "badge",
          "slot": "prefix",
          "fields": {{
            "score": {{
              "source": {{ "kind": "literal", "value": {number} }},
              "type": "float"
            }}
          }}
        }}
      }}
    }}
  }}
}}"#
    );
    load_schema_manifest_str("projection-numeric.json", &source)
}

#[test]
fn json_float_lexemes_survive_parsing_and_fingerprinting() {
    for (token, expected) in [
        ("1.0", "1.0"),
        ("-0.0", "-0.0"),
        ("1e+2", "1e+2"),
        ("1e308", "1e+308"),
    ] {
        let schema = schema_for(token);
        assert_eq!(
            schema.conditions["ready"]
                .availability_reason
                .as_ref()
                .expect("mapping")
                .args["value"],
            AvailabilityReasonArgBinding::Literal(SchemaLiteralValue::Float(expected.to_owned()))
        );
    }
    assert_ne!(
        canonical_schema_fingerprint(&schema_for("1.0")),
        canonical_schema_fingerprint(&schema_for("1")),
    );
    assert_ne!(
        canonical_schema_fingerprint(&schema_for("-0.0")),
        canonical_schema_fingerprint(&schema_for("0")),
    );
}

#[test]
fn json_projection_output_lexemes_survive_without_inputs() {
    let schema = projection_schema_for("1.0");
    assert_eq!(
        schema.presentation_projectors["projector"].outputs["badge"].fields["score"].source,
        recite_core::PresentationAffordanceFieldSource::Literal(SchemaLiteralValue::Float(
            "1.0".to_owned()
        ))
    );
    assert_ne!(
        canonical_schema_fingerprint(&projection_schema_for("1.0")),
        canonical_schema_fingerprint(&projection_schema_for("1")),
    );
}

#[test]
fn json_out_of_range_float_is_rejected_during_schema_loading() {
    let report = load_schema_manifest_str(
        "out-of-range.json",
        &format!(
            r#"{{
  "schema_version": 1,
  "conditions": {{ "ready": {{ "returns": "bool", "availability_reason": {{
    "reason": "reason", "args": {{ "value": {} }}
  }} }} }},
  "availability_reasons": {{ "reason": {{
    "template": "{{value}}", "params": [{{ "name": "value", "type": "float" }}]
  }} }}
}}"#,
            "1e400"
        ),
    );
    assert!(report.schema.is_none());
    assert_eq!(report.diagnostics.len(), 2, "{report:?}");
    let diagnostic = &report.diagnostics[0];
    assert_eq!(
        diagnostic
            .presentation
            .as_ref()
            .unwrap_or_else(|| panic!("float range diagnostic presentation"))
            .id()
            .as_str(),
        "diagnostic-schema-001-float-not-representable"
    );
    diagnostic
        .record()
        .unwrap_or_else(|error| panic!("float range diagnostic should be recordable: {error}"));
}

#[test]
fn json_projection_out_of_range_float_is_rejected_during_schema_loading() {
    let report = projection_report_for("1e400");
    assert!(report.schema.is_none());
    assert_eq!(report.diagnostics.len(), 1, "{report:?}");
    assert_eq!(
        report.diagnostics[0]
            .presentation
            .as_ref()
            .unwrap_or_else(|| panic!("float range diagnostic presentation"))
            .id()
            .as_str(),
        "diagnostic-schema-001-float-not-representable"
    );
}
