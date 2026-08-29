use recite_core::{
    AvailabilityReasonArgBinding, SchemaLiteralValue, canonical_schema_fingerprint,
    load_schema_manifest_str, load_schema_source_str,
};

fn toml_schema_for(token: &str) -> recite_core::SchemaSource {
    let source = format!(
        r#"schema_version = 1

[producer]
id = "dialogue"

[availability_reasons.reason]
template = "{{value}}"
params = [{{ name = "value", type = "float" }}]

[conditions.ready]
returns = "bool"

[conditions.ready.availability_reason]
reason = "reason"

[conditions.ready.availability_reason.args.value]
kind = "literal"
value = {token}
"#
    );
    load_schema_source_str("numeric.toml", &source)
        .source
        .unwrap_or_else(|| panic!("numeric source should load: {token}"))
}

fn json_schema_for(token: &str) -> recite_core::ProjectSchema {
    let source = format!(
        r#"{{
  "schema_version": 1,
  "conditions": {{ "ready": {{ "returns": "bool", "availability_reason": {{
    "reason": "reason", "args": {{ "value": {token} }}
  }} }} }},
  "availability_reasons": {{ "reason": {{
    "template": "{{value}}", "params": [{{ "name": "value", "type": "float" }}]
  }} }}
}}"#
    );
    load_schema_manifest_str("numeric.json", &source)
        .schema
        .unwrap_or_else(|| panic!("numeric manifest should load: {token}"))
}

fn toml_projection_schema_for(token: &str) -> recite_core::SchemaSource {
    let source = format!(
        r#"schema_version = 1

[producer]
id = "dialogue"

[presentation_projectors.projector]
candidates = {{ kind = "runtime_event", event = "dialogue" }}

[[presentation_projectors.projector.inputs]]
name = "score"
source = {{ kind = "literal", value = {token} }}
type = "float"

[presentation_projectors.projector.outputs.badge]
target = "candidate"
kind = "badge"
slot = "prefix"

[presentation_projectors.projector.outputs.badge.fields.score]
source = {{ kind = "literal", value = {token} }}
type = "float"
"#
    );
    load_schema_source_str("projection-numeric.toml", &source)
        .source
        .unwrap_or_else(|| panic!("projection source should load: {token}"))
}

fn reason_value(schema: &recite_core::ProjectSchema) -> &AvailabilityReasonArgBinding {
    match schema.conditions["ready"].availability_reason.as_ref() {
        Some(mapping) => &mapping.args["value"],
        None => panic!("availability mapping"),
    }
}

#[test]
fn toml_float_tokens_match_json_canonical_values() {
    for (toml_token, json_token, canonical) in [
        ("1.0", "1.0", "1.0"),
        ("-0.0", "-0.0", "-0.0"),
        ("1e+2", "1e+2", "1e+2"),
        ("1e2", "1e2", "1e+2"),
        ("1E2", "1E2", "1e+2"),
        ("+1e2", "1e2", "1e+2"),
        ("-1e-2", "-1e-2", "-1e-2"),
        ("-1E-2", "-1E-2", "-1e-2"),
        (
            "1.23456789012345678901234567890",
            "1.23456789012345678901234567890",
            "1.23456789012345678901234567890",
        ),
    ] {
        let toml = toml_schema_for(toml_token);
        let json = json_schema_for(json_token);
        assert_eq!(
            reason_value(toml.schema()),
            &AvailabilityReasonArgBinding::Literal(SchemaLiteralValue::Float(canonical.to_owned())),
            "{toml_token}"
        );
        assert_eq!(
            reason_value(toml.schema()),
            reason_value(&json),
            "{toml_token}"
        );
        assert_eq!(
            canonical_schema_fingerprint(toml.schema()),
            canonical_schema_fingerprint(&json),
            "{toml_token}"
        );
    }
}

#[test]
fn toml_float_exponent_spelling_canonicalizes_for_export_and_reload() {
    for (toml_token, json_token, canonical) in [
        ("1e2", "1e2", "1e+2"),
        ("1E2", "1E2", "1e+2"),
        ("+1e2", "1e2", "1e+2"),
        ("-1e-2", "-1e-2", "-1e-2"),
        ("-1E-2", "-1E-2", "-1e-2"),
    ] {
        let source = toml_schema_for(toml_token);
        let json = json_schema_for(json_token);
        assert_eq!(
            reason_value(source.schema()),
            reason_value(&json),
            "{toml_token}"
        );
        let exported = source.export_json();
        assert!(exported.contains(canonical), "{toml_token}: {exported}");
        let reloaded = load_schema_manifest_str("numeric.json", &exported);
        assert!(
            reloaded.diagnostics.is_empty(),
            "{toml_token}: {reloaded:?}"
        );
        let reloaded = reloaded.schema.expect("exported schema");
        assert_eq!(
            reason_value(source.schema()),
            reason_value(&reloaded),
            "{toml_token}"
        );
        assert_eq!(
            canonical_schema_fingerprint(source.schema()),
            canonical_schema_fingerprint(&reloaded),
            "{toml_token}"
        );
        assert_eq!(
            canonical_schema_fingerprint(source.schema()),
            canonical_schema_fingerprint(&json),
            "{toml_token}"
        );
    }
}

#[test]
fn toml_float_tokens_survive_export_and_reload() {
    for token in ["1.0", "-0.0", "1e+2", "1.23456789012345678901234567890"] {
        let source = toml_schema_for(token);
        let exported = source.export_json();
        assert!(exported.contains(token), "{token}: {exported}");
        let reloaded = load_schema_manifest_str("numeric.json", &exported);
        assert!(reloaded.diagnostics.is_empty(), "{token}: {reloaded:?}");
        let reloaded = reloaded.schema.expect("exported schema");
        assert_eq!(
            reason_value(source.schema()),
            reason_value(&reloaded),
            "{token}"
        );
        assert_eq!(
            canonical_schema_fingerprint(source.schema()),
            canonical_schema_fingerprint(&reloaded),
            "{token}"
        );
    }
}

#[test]
fn toml_float_syntax_normalizes_for_numeric_json_export() {
    for (token, normalized) in [("1_234.5_0", "1234.50"), ("+1.0", "1.0")] {
        let source = toml_schema_for(token);
        let exported = source.export_json();
        assert!(exported.contains(normalized), "{token}: {exported}");
        assert!(
            !exported.contains(&format!("\"{token}\"")),
            "{token}: {exported}"
        );
        let json = json_schema_for(normalized);
        assert_eq!(
            canonical_schema_fingerprint(source.schema()),
            canonical_schema_fingerprint(&json),
            "{token}"
        );
        let reloaded = load_schema_manifest_str("numeric.json", &exported);
        assert!(reloaded.diagnostics.is_empty(), "{token}: {reloaded:?}");
    }
}

#[test]
fn toml_projection_float_tokens_survive_export_and_reload() {
    for (token, canonical) in [
        ("1.0", "1.0"),
        ("-0.0", "-0.0"),
        ("1e+2", "1e+2"),
        (
            "1.23456789012345678901234567890",
            "1.23456789012345678901234567890",
        ),
    ] {
        let source = toml_projection_schema_for(token);
        let projector = &source.schema().presentation_projectors["projector"];
        assert_eq!(
            projector.inputs[0].source,
            recite_core::SchemaProjectionInputSource::Literal(SchemaLiteralValue::Float(
                canonical.to_owned()
            )),
            "{token}"
        );
        assert_eq!(
            projector.outputs["badge"].fields["score"].source,
            recite_core::PresentationAffordanceFieldSource::Literal(SchemaLiteralValue::Float(
                canonical.to_owned()
            )),
            "{token}"
        );

        let exported = source.export_json();
        assert!(exported.contains(canonical), "{token}: {exported}");
        let reloaded = load_schema_manifest_str("projection-numeric.json", &exported);
        assert!(reloaded.diagnostics.is_empty(), "{token}: {reloaded:?}");
        let reloaded = reloaded.schema.expect("exported projection schema");
        assert_eq!(
            canonical_schema_fingerprint(source.schema()),
            canonical_schema_fingerprint(&reloaded),
            "{token}"
        );
    }
}

#[test]
fn toml_projection_float_syntax_normalizes_for_numeric_json_export() {
    for (token, normalized) in [("1_234.5_0", "1234.50"), ("+1.0", "1.0")] {
        let source = toml_projection_schema_for(token);
        let projector = &source.schema().presentation_projectors["projector"];
        assert_eq!(
            projector.inputs[0].source,
            recite_core::SchemaProjectionInputSource::Literal(SchemaLiteralValue::Float(
                normalized.to_owned()
            )),
            "{token}"
        );
        assert_eq!(
            projector.outputs["badge"].fields["score"].source,
            recite_core::PresentationAffordanceFieldSource::Literal(SchemaLiteralValue::Float(
                normalized.to_owned()
            )),
            "{token}"
        );
        let exported = source.export_json();
        assert!(exported.contains(normalized), "{token}: {exported}");
        assert!(
            !exported.contains(&format!("\"{token}\"")),
            "{token}: {exported}"
        );
        let reloaded = load_schema_manifest_str("projection-numeric.json", &exported);
        assert!(reloaded.diagnostics.is_empty(), "{token}: {reloaded:?}");
        assert_eq!(
            canonical_schema_fingerprint(source.schema()),
            canonical_schema_fingerprint(
                &reloaded.schema.unwrap_or_else(|| {
                    panic!("exported projection schema should load: {token}")
                })
            ),
            "{token}"
        );
    }
}

#[test]
fn non_finite_toml_float_tokens_remain_rejected_before_lowering() {
    for (token, expected_id) in [
        ("nan", "diagnostic-schema-001-source-non-finite"),
        ("inf", "diagnostic-schema-001-source-non-finite"),
        ("-inf", "diagnostic-schema-001-source-non-finite"),
        ("1e400", "diagnostic-schema-001-toml-parse"),
    ] {
        let source = format!(
            "schema_version = 1\n[producer]\nid = \"dialogue\"\n[availability_reasons.reason]\ntemplate = \"{{value}}\"\nparams = [{{ name = \"value\", type = \"float\" }}]\n[conditions.ready]\nreturns = \"bool\"\n[conditions.ready.availability_reason]\nreason = \"reason\"\n[conditions.ready.availability_reason.args.value]\nkind = \"literal\"\nvalue = {token}\n"
        );
        let report = load_schema_source_str("non-finite.toml", &source);
        assert!(report.source.is_none(), "{token}");
        assert_eq!(report.diagnostics.len(), 1, "{token}: {report:?}");
        assert_eq!(
            report.diagnostics[0]
                .presentation
                .as_ref()
                .expect("diagnostic presentation")
                .id()
                .as_str(),
            expected_id,
            "{token}"
        );
    }
}
