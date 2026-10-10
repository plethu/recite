use proptest::prelude::*;
use recite_core::schema::{
    SchemaLiteralValue, SchemaProjectionInputSource, load_schema_manifest_str,
};

fn manifest(type_ref: &str, value: &str) -> String {
    format!(
        r#"{{
  "schema_version": 1,
  "types": {{"mood": {{"kind": "enum", "values": ["calm"]}}}},
  "registries": {{"item": {{"values": ["key"]}}}},
  "speakers": {{"rhea": {{}}}},
  "presentation_projectors": {{"p": {{
    "candidates": {{"kind": "runtime_event", "event": "dialogue"}},
    "inputs": [{{"name": "value", "source": {{"kind": "literal", "value": {value}}}, "type": "{type_ref}"}}],
    "queries": {{}}, "outputs": {{}}
  }}}}
}}"#
    )
}

#[test]
fn typed_literals_reject_wrong_shapes_and_unknown_domain_members() {
    for (type_ref, value, expected) in [
        ("string", "null", "type"),
        ("string", "true", "type"),
        ("string", "42", "type"),
        ("string", "[]", "type"),
        ("string", "{}", "type"),
        ("bool", "\"true\"", "type"),
        ("int", "1.5", "int"),
        ("int", "9223372036854775808", "int"),
        ("int", "-9223372036854775809", "int"),
        ("float", "1e400", "finite-float"),
        ("speaker", "\"unknown\"", "unknown"),
        ("enum:mood", "\"unknown\"", "unknown"),
        ("registry:item", "\"unknown\"", "unknown"),
        ("speaker", "1", "type"),
        ("enum:mood", "false", "type"),
        ("registry:item", "null", "type"),
    ] {
        let report = load_schema_manifest_str("literal.json", &manifest(type_ref, value));
        assert!(report.schema.is_none(), "{type_ref} must reject {value}");
        assert_eq!(
            report.diagnostics.len(),
            1,
            "{type_ref}/{value}: {:?}",
            report.diagnostics
        );
        let diagnostic = &report.diagnostics[0];
        let id = diagnostic
            .presentation
            .as_ref()
            .expect("structured presentation")
            .id()
            .as_str();
        let expected = if expected == "finite-float" {
            "diagnostic-schema-001-float-not-representable".to_owned()
        } else {
            format!("diagnostic-schema-001-projection-literal-{expected}")
        };
        assert_eq!(id, expected, "{type_ref}/{value}");
        assert!(diagnostic.record().is_ok());
    }
}

#[test]
fn typed_literals_preserve_valid_domain_members_and_boolean_values() {
    for (type_ref, value, literal) in [
        (
            "speaker",
            "\"rhea\"",
            SchemaLiteralValue::String("rhea".to_owned()),
        ),
        (
            "enum:mood",
            "\"calm\"",
            SchemaLiteralValue::String("calm".to_owned()),
        ),
        (
            "registry:item",
            "\"key\"",
            SchemaLiteralValue::String("key".to_owned()),
        ),
        ("bool", "true", SchemaLiteralValue::Bool(true)),
        ("bool", "false", SchemaLiteralValue::Bool(false)),
    ] {
        let report = load_schema_manifest_str("literal.json", &manifest(type_ref, value));
        assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
        let schema = report.schema.expect("valid literal");
        assert_eq!(
            schema.presentation_projectors["p"].inputs[0].source,
            SchemaProjectionInputSource::Literal(literal)
        );
    }
}

proptest! {
    #[test]
    fn integer_projection_literals_preserve_the_full_signed_domain(value in any::<i64>()) {
        let report = load_schema_manifest_str("literal.json", &manifest("int", &value.to_string()));
        prop_assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
        let schema = report.schema.ok_or_else(|| TestCaseError::fail("integer must lower"))?;
        prop_assert_eq!(&schema.presentation_projectors["p"].inputs[0].source,
            &SchemaProjectionInputSource::Literal(SchemaLiteralValue::Int(value)));
    }

    #[test]
    fn string_projection_literals_preserve_unicode_without_binding_reinterpretation(value in ".{0,48}") {
        let json = serde_json::to_string(&value).map_err(|error| TestCaseError::fail(error.to_string()))?;
        let report = load_schema_manifest_str("literal.json", &manifest("string", &json));
        prop_assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
        let schema = report.schema.ok_or_else(|| TestCaseError::fail("string must lower"))?;
        prop_assert_eq!(&schema.presentation_projectors["p"].inputs[0].source,
            &SchemaProjectionInputSource::Literal(SchemaLiteralValue::String(value)));
    }
}
