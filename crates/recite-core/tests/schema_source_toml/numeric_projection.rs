use recite_core::{
    PresentationAffordanceFieldSource, SchemaLiteralValue, SchemaProjectionInputSource,
    canonical_schema_fingerprint, load_schema_manifest_str, load_schema_source_str,
};

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

fn json_projection_schema_for(token: &str) -> recite_core::ProjectSchema {
    let source = format!(
        r#"{{
  "schema_version": 1,
  "presentation_projectors": {{
    "projector": {{
      "candidates": {{ "kind": "runtime_event", "event": "dialogue" }},
      "inputs": [{{
        "name": "score",
        "source": {{ "kind": "literal", "value": {token} }},
        "type": "float"
      }}],
      "outputs": {{
        "badge": {{
          "target": "candidate",
          "kind": "badge",
          "slot": "prefix",
          "fields": {{
            "score": {{
              "source": {{ "kind": "literal", "value": {token} }},
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
        .schema
        .unwrap_or_else(|| panic!("projection manifest should load: {token}"))
}

#[test]
fn toml_projection_float_exponents_match_json_canonical_values() {
    for (toml_token, json_token, canonical) in [
        ("1e2", "1e2", "1e+2"),
        ("1E2", "1E2", "1e+2"),
        ("+1e2", "1e2", "1e+2"),
        ("-1e-2", "-1e-2", "-1e-2"),
        ("-1E-2", "-1E-2", "-1e-2"),
    ] {
        let source = toml_projection_schema_for(toml_token);
        let json = json_projection_schema_for(json_token);
        let projector = &source.schema().presentation_projectors["projector"];
        assert_eq!(
            projector.inputs[0].source,
            SchemaProjectionInputSource::Literal(SchemaLiteralValue::Float(canonical.to_owned())),
            "{toml_token}"
        );
        assert_eq!(
            projector.outputs["badge"].fields["score"].source,
            PresentationAffordanceFieldSource::Literal(SchemaLiteralValue::Float(
                canonical.to_owned()
            )),
            "{toml_token}"
        );
        assert_eq!(
            canonical_schema_fingerprint(source.schema()),
            canonical_schema_fingerprint(&json),
            "{toml_token}"
        );

        let exported = source.export_json();
        assert!(exported.contains(canonical), "{toml_token}: {exported}");
        let reloaded = load_schema_manifest_str("projection-numeric.json", &exported);
        assert!(
            reloaded.diagnostics.is_empty(),
            "{toml_token}: {reloaded:?}"
        );
        let reloaded = reloaded.schema.expect("exported projection schema");
        assert_eq!(
            canonical_schema_fingerprint(source.schema()),
            canonical_schema_fingerprint(&reloaded),
            "{toml_token}"
        );
    }
}
