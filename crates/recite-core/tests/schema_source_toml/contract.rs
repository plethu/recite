use recite_core::{load_schema_manifest_str, load_schema_source_str};

use crate::assert_recordable_diagnostics;

#[test]
fn minimal_source_loads() {
    let source_text = r#"
schema_version = 1

[producer]
id = "dialogue"

[types.actor]
kind = "enum"
values = ["player"]
"#;
    let report = load_schema_source_str("schema.toml", source_text);
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    let source = report.source.expect("valid source");
    assert_eq!(source.source_text(), source_text);
    assert_eq!(source.schema().types.len(), 1);

    let generated = source.export_json();
    let generated_report = load_schema_manifest_str("schema.json", &generated);
    assert!(
        generated_report.diagnostics.is_empty(),
        "{:?}",
        generated_report.diagnostics
    );
    assert_eq!(generated_report.schema, Some(source.schema().clone()));
}

#[test]
fn tagged_reason_bindings_and_literals_lower_through_manifest_rules() {
    let source = r#"
schema_version = 1

[producer]
id = "dialogue"

[speakers.actor]

[types.mood]
kind = "enum"
values = ["calm", "tense"]

[availability_reasons.not_ready]
template = "{speaker} is {mood}"
params = [
  { name = "speaker", type = "speaker" },
  { name = "mood", type = "enum:mood" },
]

[conditions.ready]
params = [
  { name = "actor", type = "speaker" },
]
returns = "bool"

[conditions.ready.availability_reason]
reason = "not_ready"

[conditions.ready.availability_reason.args.speaker]
kind = "binding"
name = "actor"

[conditions.ready.availability_reason.args.mood]
kind = "literal"
value = "calm"
"#;
    let report = load_schema_source_str("schema.toml", source);
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    let schema = report.source.expect("valid source").schema().clone();
    let mapping = schema
        .conditions
        .get("ready")
        .and_then(|condition| condition.availability_reason.as_ref())
        .expect("availability mapping");
    assert!(matches!(
        mapping.args.get("speaker"),
        Some(recite_core::AvailabilityReasonArgBinding::ConditionParam(name))
            if name == "actor"
    ));
    assert!(matches!(
        mapping.args.get("mood"),
        Some(recite_core::AvailabilityReasonArgBinding::Literal(
            recite_core::SchemaLiteralValue::String(value)
        )) if value == "calm"
    ));
}

#[test]
fn tagged_literal_dollar_value_is_not_binding_shorthand() {
    let source = r#"schema_version = 1
[producer]
id = "dialogue"
[availability_reasons.reason]
template = "{value}"
params = [{ name = "value", type = "string" }]
[conditions.ready]
returns = "bool"
[conditions.ready.availability_reason]
reason = "reason"
[conditions.ready.availability_reason.args.value]
kind = "literal"
value = "$literal"
"#;
    let report = load_schema_source_str("literal-dollar.toml", source);
    let source = report.source.expect("explicit literal is valid");
    let schema = source.schema();
    assert_eq!(
        schema.conditions["ready"]
            .availability_reason
            .as_ref()
            .expect("mapping")
            .args["value"],
        recite_core::AvailabilityReasonArgBinding::Literal(
            recite_core::SchemaLiteralValue::String("$literal".to_owned())
        )
    );

    let exported = source.export_json();
    assert!(exported.contains("\"$$literal\""), "{exported}");
    let generated = load_schema_manifest_str("literal-dollar.json", &exported);
    assert!(
        generated.diagnostics.is_empty(),
        "{:?}",
        generated.diagnostics
    );
    assert_eq!(generated.schema, Some(schema.clone()));
}

#[test]
fn toml_rejects_json_only_dollar_binding_shorthand() {
    let source = r#"
schema_version = 1
[producer]
id = "dialogue"
[availability_reasons.not_ready]
template = "{actor} is not ready"
params = [{ name = "actor", type = "speaker" }]
[conditions.ready]
params = [{ name = "actor", type = "speaker" }]
returns = "bool"
[conditions.ready.availability_reason]
reason = "not_ready"
[conditions.ready.availability_reason.args.actor]
"#;
    let source = format!("{source}value = \"$actor\"\n");
    let report = load_schema_source_str("legacy-binding.toml", &source);
    assert!(report.source.is_none());
    assert_eq!(report.diagnostics.len(), 1);
    assert_eq!(
        report.diagnostics[0]
            .presentation
            .as_ref()
            .expect("legacy-binding presentation")
            .id()
            .as_str(),
        "diagnostic-schema-001-source-legacy-binding"
    );
    assert_recordable_diagnostics(&report);
}

#[test]
fn tagged_availability_objects_are_toml_only() {
    let json = r#"{
  "schema_version": 1,
  "conditions": { "ready": { "returns": "bool", "availability_reason": {
    "reason": "reason", "args": { "value": { "kind": "literal", "value": "ok" } }
  } } },
  "availability_reasons": { "reason": {
    "template": "{value}", "params": [{ "name": "value", "type": "string" }]
  } }
}"#;
    let report = load_schema_manifest_str("tagged.json", json);
    assert!(report.schema.is_none());
    assert_eq!(report.diagnostics.len(), 2, "{report:?}");
    for diagnostic in &report.diagnostics {
        assert!(diagnostic.presentation.is_some());
        assert!(diagnostic.related.is_empty());
        assert!(diagnostic.help.is_none());
        diagnostic
            .record()
            .expect("tagged-only-TOML diagnostic is recordable");
    }
    assert_eq!(
        report.diagnostics[0]
            .presentation
            .as_ref()
            .expect("tagged-only-TOML presentation")
            .id()
            .as_str(),
        "diagnostic-schema-001-availability-tagged-only-toml"
    );
}

#[test]
fn standalone_fixture_covers_declarations_and_provenance() {
    let report = load_schema_source_str(
        "fixtures/schema/valid/standalone.toml",
        include_str!("../../../../fixtures/schema/valid/standalone.toml"),
    );
    assert!(report.diagnostics.is_empty(), "{:?}", report.diagnostics);
    let source = report.source.expect("valid standalone fixture");
    let exported = source.export_json();
    let generated = load_schema_manifest_str("standalone.json", &exported);
    assert!(
        generated.diagnostics.is_empty(),
        "{:?}",
        generated.diagnostics
    );
    assert_eq!(generated.schema, Some(source.schema().clone()));
}
