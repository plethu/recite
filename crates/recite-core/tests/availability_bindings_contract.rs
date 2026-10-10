use proptest::prelude::*;
use recite_core::schema::{
    AvailabilityReasonArgBinding, SchemaLiteralValue, load_schema_manifest_str,
};

fn manifest(type_ref: &str, value: &str, parameter_type: &str) -> String {
    format!(
        r#"{{
      "schema_version": 1,
      "types": {{"mood": {{"kind": "enum", "values": ["calm"]}}}},
      "registries": {{"item": {{"values": ["key"]}}}},
      "speakers": {{"rhea": {{}}}},
      "availability_reasons": {{"blocked": {{"template": "{{value}}", "params": [{{"name": "value", "type": "{type_ref}"}}]}}}},
      "conditions": {{"ready": {{"returns": "bool", "params": [{{"name": "binding", "type": "{parameter_type}"}}],
        "availability_reason": {{"reason": "blocked", "args": {{"value": {value}}}}}
      }}}}
    }}"#
    )
}

#[test]
fn malformed_reason_literals_are_rejected_at_the_binding_owner() {
    for (type_ref, value, expected) in [
        (
            "string",
            "null",
            "diagnostic-schema-001-availability-binding-literal-type",
        ),
        (
            "string",
            "false",
            "diagnostic-schema-001-availability-binding-literal-type",
        ),
        (
            "string",
            "1",
            "diagnostic-schema-001-availability-binding-literal-type",
        ),
        (
            "string",
            "[]",
            "diagnostic-schema-001-availability-binding-literal-type",
        ),
        (
            "string",
            "{}",
            "diagnostic-schema-001-availability-tagged-only-toml",
        ),
        (
            "int",
            "\"one\"",
            "diagnostic-schema-001-availability-binding-string-type",
        ),
        (
            "bool",
            "\"true\"",
            "diagnostic-schema-001-availability-binding-string-type",
        ),
        (
            "float",
            "\"1.5\"",
            "diagnostic-schema-001-availability-binding-string-type",
        ),
        (
            "int",
            "1.5",
            "diagnostic-schema-001-availability-binding-int",
        ),
        (
            "int",
            "9223372036854775808",
            "diagnostic-schema-001-availability-binding-int",
        ),
        (
            "float",
            "1e400",
            "diagnostic-schema-001-float-not-representable",
        ),
        (
            "speaker",
            "\"unknown\"",
            "diagnostic-schema-001-availability-binding-unknown-value",
        ),
        (
            "enum:mood",
            "\"unknown\"",
            "diagnostic-schema-001-availability-binding-unknown-value",
        ),
        (
            "registry:item",
            "\"unknown\"",
            "diagnostic-schema-001-availability-binding-unknown-value",
        ),
        (
            "int",
            "\"$binding\"",
            "diagnostic-schema-001-availability-binding-type-mismatch",
        ),
        (
            "string",
            "\"$missing\"",
            "diagnostic-schema-004-unknown-condition-param",
        ),
    ] {
        let report = load_schema_manifest_str("binding.json", &manifest(type_ref, value, "string"));
        assert!(report.schema.is_none(), "{type_ref}/{value}");
        assert_eq!(
            report.diagnostics.len(),
            2,
            "{type_ref}/{value}: {:?}",
            report.diagnostics
        );
        let diagnostic = &report.diagnostics[0];
        assert_eq!(
            diagnostic
                .presentation
                .as_ref()
                .expect("structured presentation")
                .id()
                .as_str(),
            expected
        );
        assert!(diagnostic.record().is_ok());
        assert_eq!(
            report.diagnostics[1]
                .presentation
                .as_ref()
                .expect("missing argument presentation")
                .id()
                .as_str(),
            "diagnostic-schema-001-availability-missing-reason-arg"
        );
        assert!(report.diagnostics[1].record().is_ok());
    }
}

#[test]
fn typed_reason_literal_domains_and_parameter_references_preserve_their_meaning() {
    for (type_ref, value, expected) in [
        (
            "speaker",
            "\"rhea\"",
            AvailabilityReasonArgBinding::Literal(SchemaLiteralValue::String("rhea".to_owned())),
        ),
        (
            "enum:mood",
            "\"calm\"",
            AvailabilityReasonArgBinding::Literal(SchemaLiteralValue::String("calm".to_owned())),
        ),
        (
            "registry:item",
            "\"key\"",
            AvailabilityReasonArgBinding::Literal(SchemaLiteralValue::String("key".to_owned())),
        ),
        (
            "bool",
            "true",
            AvailabilityReasonArgBinding::Literal(SchemaLiteralValue::Bool(true)),
        ),
        (
            "float",
            "1.25",
            AvailabilityReasonArgBinding::Literal(SchemaLiteralValue::Float("1.25".to_owned())),
        ),
        (
            "string",
            "\"$binding\"",
            AvailabilityReasonArgBinding::ConditionParam("binding".to_owned()),
        ),
    ] {
        let report = load_schema_manifest_str("binding.json", &manifest(type_ref, value, "string"));
        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
        let schema = report.schema.expect("valid binding");
        let mapping = schema.conditions["ready"]
            .availability_reason
            .as_ref()
            .expect("reason mapping");
        assert_eq!(mapping.args["value"], expected);
    }
}

proptest! {
    #[test]
    fn escaped_reason_string_literals_round_trip_without_becoming_parameters(value in ".{0,48}") {
        let escaped = if value.starts_with('$') { format!("${value}") } else { value.clone() };
        let json = serde_json::to_string(&escaped).map_err(|error| TestCaseError::fail(error.to_string()))?;
        let report = load_schema_manifest_str("binding.json", &manifest("string", &json, "string"));
        prop_assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
        let schema = report.schema.ok_or_else(|| TestCaseError::fail("string literal must lower"))?;
        let mapping = schema.conditions["ready"].availability_reason.as_ref()
            .ok_or_else(|| TestCaseError::fail("reason mapping must be retained"))?;
        prop_assert_eq!(&mapping.args["value"],
            &AvailabilityReasonArgBinding::Literal(SchemaLiteralValue::String(value)));
    }
}
