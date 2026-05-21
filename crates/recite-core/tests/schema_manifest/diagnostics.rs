use recite_core::load_schema_manifest_str;

use crate::diagnostic_codes;

#[test]
fn malformed_manifest_shape_reports_schema_diagnostic() {
    let report = load_schema_manifest_str(
        "fixtures/schema/invalid/malformed_shape.json",
        include_str!("../../../../fixtures/schema/invalid/malformed_shape.json"),
    );

    assert!(report.schema.is_none());
    assert_eq!(diagnostic_codes(&report), ["RECITE_SCHEMA001"]);
    assert_eq!(report.diagnostics[0].span.start.line(), 2);
}

#[test]
fn unsupported_manifest_versions_report_schema_diagnostic() {
    let report = load_schema_manifest_str(
        "fixtures/schema/invalid/unsupported_version.json",
        include_str!("../../../../fixtures/schema/invalid/unsupported_version.json"),
    );

    assert!(report.schema.is_none());
    assert_eq!(diagnostic_codes(&report), ["RECITE_SCHEMA002"]);
}

#[test]
fn duplicate_definitions_and_values_report_stable_diagnostics() {
    let duplicate_definitions = load_schema_manifest_str(
        "fixtures/schema/invalid/duplicate_definitions.json",
        include_str!("../../../../fixtures/schema/invalid/duplicate_definitions.json"),
    );
    assert!(duplicate_definitions.schema.is_none());
    assert_eq!(
        diagnostic_codes(&duplicate_definitions),
        ["RECITE_SCHEMA003"]
    );
    assert_eq!(duplicate_definitions.diagnostics[0].span.start.line(), 8);

    let duplicate_values = load_schema_manifest_str(
        "fixtures/schema/invalid/duplicate_values.json",
        include_str!("../../../../fixtures/schema/invalid/duplicate_values.json"),
    );
    assert!(duplicate_values.schema.is_none());
    assert_eq!(
        diagnostic_codes(&duplicate_values),
        ["RECITE_SCHEMA003", "RECITE_SCHEMA003"]
    );
    assert_eq!(duplicate_values.diagnostics[0].span.start.line(), 6);
    assert_eq!(duplicate_values.diagnostics[1].span.start.line(), 11);
}

#[test]
fn invalid_enum_and_registry_type_references_report_stable_diagnostics() {
    let report = load_schema_manifest_str(
        "fixtures/schema/invalid/invalid_type_references.json",
        include_str!("../../../../fixtures/schema/invalid/invalid_type_references.json"),
    );

    assert!(report.schema.is_none());
    assert_eq!(
        diagnostic_codes(&report),
        [
            "RECITE_SCHEMA004",
            "RECITE_SCHEMA004",
            "RECITE_SCHEMA004",
            "RECITE_SCHEMA004"
        ]
    );
    assert_eq!(
        report.diagnostics[0].message,
        "condition 'thread_stage' parameter 'thread_id' references unknown registry 'thread'"
    );
    assert_eq!(report.diagnostics[0].span.start.line(), 5);
    assert_eq!(report.diagnostics[1].span.start.line(), 6);
    assert_eq!(report.diagnostics[2].span.start.line(), 12);
    assert_eq!(report.diagnostics[3].span.start.line(), 18);
}

#[test]
fn manifest_loader_rejects_strings_rejected_by_the_public_json_schema_contract() {
    let report = load_schema_manifest_str(
        "fixtures/schema/invalid/schema_contract_drift.json",
        r#"{
  "schema_version": 1,
  "types": {
    "state": { "kind": "enum", "values": [""] }
  },
  "registries": {
    "sound": { "values": ["snap"], "origin": "" }
  },
  "speakers": {
    "hazel": { "display_name": "" }
  },
  "conditions": {
    "bad_condition": {
      "params": [{ "name": "", "type": "enum:bad space" }],
      "returns": "enum:"
    }
  },
  "metadata": {
    "bad_metadata": { "targets": ["line"], "type": "registry:" }
  }
}"#,
    );

    assert!(report.schema.is_none());
    assert_eq!(
        diagnostic_codes(&report),
        [
            "RECITE_SCHEMA001",
            "RECITE_SCHEMA001",
            "RECITE_SCHEMA001",
            "RECITE_SCHEMA001",
            "RECITE_SCHEMA004",
            "RECITE_SCHEMA004",
            "RECITE_SCHEMA004"
        ]
    );
}

#[test]
fn manifest_loader_rejects_missing_required_values_fields() {
    let report = load_schema_manifest_str(
        "fixtures/schema/invalid/missing_required_values.json",
        r#"{
  "schema_version": 1,
  "types": {
    "state": { "kind": "enum" }
  }
}"#,
    );

    assert!(report.schema.is_none());
    assert_eq!(diagnostic_codes(&report), ["RECITE_SCHEMA001"]);
}

#[test]
fn manifest_loader_rejects_explicit_null_optionals() {
    let report = load_schema_manifest_str(
        "fixtures/schema/invalid/null_optionals.json",
        r#"{
  "schema_version": 1,
  "registries": {
    "sound": { "values": ["snap"], "origin": null }
  }
}"#,
    );

    assert!(report.schema.is_none());
    assert_eq!(diagnostic_codes(&report), ["RECITE_SCHEMA001"]);
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
fn duplicate_parameter_names_are_schema_diagnostics() {
    let report = load_schema_manifest_str(
        "fixtures/schema/invalid/duplicate_params.json",
        r#"{
  "schema_version": 1,
  "conditions": {
    "trust_gte": {
      "params": [
        { "name": "actor", "type": "speaker" },
        { "name": "actor", "type": "speaker" }
      ]
    }
  }
}"#,
    );

    assert!(report.schema.is_none());
    assert_eq!(diagnostic_codes(&report), ["RECITE_SCHEMA003"]);
    assert_eq!(report.diagnostics[0].span.start.line(), 7);
}
