use recite_core::load_schema_manifest_str;

use crate::diagnostic_codes;

#[test]
fn escaped_section_and_value_strings_keep_semantic_diagnostic_spans() {
    let report = load_schema_manifest_str(
        "fixtures/schema/invalid/escaped_section_value_spans.json",
        r#"{
  "schema_version": 1,
  "effects": {
    "bad_effect": {
      "modes": ["deferred"],
      "params": [{ "name": "target", "type": "registry:missing" }]
    }
  }
}"#,
    );

    assert!(report.schema.is_none());
    assert_eq!(diagnostic_codes(&report), ["RECITE_SCHEMA004"]);
    assert_eq!(report.diagnostics[0].span.start.line(), 6);
}

#[test]
fn diagnostic_spans_stay_with_their_section_when_top_level_objects_are_reordered() {
    let report = load_schema_manifest_str(
        "fixtures/schema/invalid/reordered_sections_repeated_refs.json",
        r#"{
  "schema_version": 1,
  "effects": {
    "bad_effect": {
      "modes": ["deferred"],
      "params": [{ "name": "target", "type": "registry:missing" }]
    }
  },
  "conditions": {
    "bad_condition": {
      "params": [{ "name": "target", "type": "registry:missing" }]
    }
  }
}"#,
    );

    assert!(report.schema.is_none());
    assert_eq!(
        diagnostic_codes(&report),
        ["RECITE_SCHEMA004", "RECITE_SCHEMA004"]
    );
    assert_eq!(report.diagnostics[0].span.start.line(), 11);
    assert_eq!(report.diagnostics[1].span.start.line(), 6);
}

#[test]
fn section_span_lookup_ignores_section_names_inside_values() {
    let report = load_schema_manifest_str(
        "fixtures/schema/invalid/section_name_value_before_section.json",
        r#"{
  "schema_version": 1,
  "registries": {
    "labels": { "values": ["effects"] }
  },
  "effects": {
    "bad_effect": {
      "modes": ["deferred"],
      "params": [{ "name": "target", "type": "registry:missing" }]
    }
  }
}"#,
    );

    assert!(report.schema.is_none());
    assert_eq!(diagnostic_codes(&report), ["RECITE_SCHEMA004"]);
    assert_eq!(report.diagnostics[0].span.start.line(), 9);
}

#[test]
fn section_span_lookup_ignores_the_section_key_itself() {
    let report = load_schema_manifest_str(
        "fixtures/schema/invalid/section_name_as_definition_name.json",
        r#"{
  "schema_version": 1,
  "effects": {
    "effects": {
      "modes": ["deferred"]
    },
    "effects": {
      "modes": ["immediate"]
    }
  }
}"#,
    );

    assert!(report.schema.is_none());
    assert_eq!(diagnostic_codes(&report), ["RECITE_SCHEMA003"]);
    assert_eq!(report.diagnostics[0].span.start.line(), 7);
}

#[test]
fn value_spans_ignore_matching_manifest_field_keys() {
    let duplicate_enum_value = load_schema_manifest_str(
        "fixtures/schema/invalid/value_name_matches_field_key.json",
        r#"{
  "schema_version": 1,
  "types": {
    "state": {
      "kind": "enum",
      "values": ["values", "values"]
    }
  }
}"#,
    );
    assert!(duplicate_enum_value.schema.is_none());
    assert_eq!(
        diagnostic_codes(&duplicate_enum_value),
        ["RECITE_SCHEMA003"]
    );
    assert_eq!(duplicate_enum_value.diagnostics[0].span.start.line(), 6);

    let duplicate_parameter_name = load_schema_manifest_str(
        "fixtures/schema/invalid/parameter_name_matches_field_key.json",
        r#"{
  "schema_version": 1,
  "conditions": {
    "check": {
      "params": [
        { "name": "params", "type": "bool" },
        { "name": "params", "type": "bool" }
      ]
    }
  }
}"#,
    );
    assert!(duplicate_parameter_name.schema.is_none());
    assert_eq!(
        diagnostic_codes(&duplicate_parameter_name),
        ["RECITE_SCHEMA003"]
    );
    assert_eq!(duplicate_parameter_name.diagnostics[0].span.start.line(), 7);
}

#[test]
fn definition_key_spans_ignore_matching_inner_field_keys() {
    let report = load_schema_manifest_str(
        "fixtures/schema/invalid/definition_key_matches_field_key.json",
        r#"{
  "schema_version": 1,
  "types": {
    "kind": { "kind": "enum", "values": ["fresh"] },
    "kind": { "kind": "enum", "values": ["stale"] }
  }
}"#,
    );

    assert!(report.schema.is_none());
    assert_eq!(diagnostic_codes(&report), ["RECITE_SCHEMA003"]);
    assert_eq!(report.diagnostics[0].span.start.line(), 5);
}
