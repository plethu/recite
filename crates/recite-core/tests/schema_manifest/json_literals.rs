use recite_core::{AvailabilityReasonArgBinding, SchemaLiteralValue, load_schema_manifest_str};

fn schema_for(value: &str) -> recite_core::ProjectSchema {
    let source = format!(
        r#"{{
  "schema_version": 1,
  "conditions": {{ "ready": {{ "params": [{{ "name": "binding", "type": "string" }}], "returns": "bool", "availability_reason": {{
    "reason": "reason", "args": {{ "value": {value} }}
  }} }} }},
  "availability_reasons": {{ "reason": {{
    "template": "{{value}}", "params": [{{ "name": "value", "type": "string" }}]
  }} }}
}}"#
    );
    load_schema_manifest_str("literal-dollar.json", &source)
        .schema
        .unwrap_or_else(|| panic!("manifest should load: {value}"))
}

#[test]
fn json_dollar_literals_keep_legacy_binding_and_literal_rules() {
    let binding = schema_for(r#""$binding""#);
    assert_eq!(
        binding.conditions["ready"]
            .availability_reason
            .as_ref()
            .expect("mapping")
            .args["value"],
        AvailabilityReasonArgBinding::ConditionParam("binding".to_owned())
    );

    let escaped = schema_for(r#""$$literal""#);
    assert_eq!(
        escaped.conditions["ready"]
            .availability_reason
            .as_ref()
            .expect("mapping")
            .args["value"],
        AvailabilityReasonArgBinding::Literal(SchemaLiteralValue::String("$literal".to_owned()))
    );

    let ordinary = schema_for(r#""ordinary""#);
    assert_eq!(
        ordinary.conditions["ready"]
            .availability_reason
            .as_ref()
            .expect("mapping")
            .args["value"],
        AvailabilityReasonArgBinding::Literal(SchemaLiteralValue::String("ordinary".to_owned()))
    );
}

#[test]
fn json_projection_literals_preserve_leading_dollars_exactly() {
    let source = r#"{
  "schema_version": 1,
  "presentation_projectors": {
    "projector": {
      "candidates": { "kind": "runtime_event", "event": "dialogue" },
      "inputs": [{
        "name": "literal",
        "source": { "kind": "literal", "value": "$$legacy" },
        "type": "string",
        "required": true
      }],
      "queries": {},
      "outputs": {}
    }
  }
}"#;
    let report = load_schema_manifest_str("projection-dollar.json", source);
    let schema = report
        .schema
        .unwrap_or_else(|| panic!("projection manifest should load: {:?}", report.diagnostics));
    assert_eq!(
        schema.presentation_projectors["projector"].inputs[0].source,
        recite_core::SchemaProjectionInputSource::Literal(SchemaLiteralValue::String(
            "$$legacy".to_owned()
        ))
    );
}
