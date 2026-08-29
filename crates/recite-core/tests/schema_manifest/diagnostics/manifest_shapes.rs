use recite_core::{DiagnosticArgumentValue, load_schema_manifest_str};

use crate::diagnostic_codes;

#[test]
fn manifest_loader_rejects_unknown_top_level_producer_fields() {
    let report = load_schema_manifest_str(
        "fixtures/schema/invalid/unknown_top_level_producer_field.json",
        include_str!(
            "../../../../../fixtures/schema/invalid/unknown_top_level_producer_field.json"
        ),
    );

    assert!(report.schema.is_none());
    assert_eq!(diagnostic_codes(&report), ["RECITE_SCHEMA001"]);
}

#[test]
fn manifest_loader_rejects_zero_export_version() {
    let report = load_schema_manifest_str(
        "fixtures/schema/invalid/invalid_export_version.json",
        include_str!("../../../../../fixtures/schema/invalid/invalid_export_version.json"),
    );

    assert!(report.schema.is_none());
    assert_eq!(diagnostic_codes(&report), ["RECITE_SCHEMA001"]);
}

#[test]
fn manifest_loader_reports_pre_v1_string_origin_migration() {
    let report = load_schema_manifest_str(
        "fixtures/schema/invalid/legacy_string_origin.json",
        r#"{
  "schema_version": 1,
  "registries": { "items": { "values": ["key"], "origin": "legacy" } }
}"#,
    );
    assert!(report.schema.is_none());
    assert_eq!(diagnostic_codes(&report), ["RECITE_SCHEMA001"]);
    let diagnostic = &report.diagnostics[0];
    assert_eq!(
        diagnostic
            .presentation
            .as_ref()
            .expect("legacy origin parse presentation")
            .id()
            .as_str(),
        "diagnostic-schema-001-json-parse"
    );
    assert!(matches!(
        diagnostic
            .presentation
            .as_ref()
            .and_then(|presentation| presentation.arguments().get("detail")),
        Some(DiagnosticArgumentValue::String(_))
    ));
}

#[test]
fn manifest_loader_rejects_phantom_provenance_keys() {
    let report = load_schema_manifest_str(
        "fixtures/schema/invalid/phantom_provenance_key.json",
        r#"{
  "schema_version": 1,
  "registries": {
    "items": {
      "values": ["key"],
      "value_origins": { "missing": { "kind": "asset", "id": "missing" } }
    }
  }
}"#,
    );
    assert!(report.schema.is_none());
    assert_eq!(diagnostic_codes(&report), ["RECITE_SCHEMA001"]);
    assert_eq!(
        report.diagnostics[0]
            .presentation
            .as_ref()
            .expect("phantom provenance presentation")
            .id()
            .as_str(),
        "diagnostic-schema-001-provenance-unknown-value"
    );
    assert_eq!(
        report.diagnostics[0]
            .presentation
            .as_ref()
            .expect("phantom provenance presentation")
            .arguments(),
        &std::collections::BTreeMap::from([
            (
                "owner".to_owned(),
                DiagnosticArgumentValue::String("registry 'items'".to_owned()),
            ),
            (
                "key".to_owned(),
                DiagnosticArgumentValue::String("missing".to_owned()),
            ),
        ])
    );
}

#[test]
fn manifest_loader_rejects_invalid_projection_declarations() {
    let report = load_schema_manifest_str(
        "fixtures/schema/invalid/presentation_projection.json",
        r#"{
  "schema_version": 1,
  "metadata": {
    "skill": {
      "targets": ["line"],
      "type": "string"
    },
    "tag": {
      "targets": ["choice"],
      "type": "symbol"
    }
  },
  "projection_queries": {
    "actor_skill": {
      "params": [{ "name": "skill", "type": "string" }],
      "returns": "int"
    }
  },
  "presentation_projectors": {
    "choice_skill_prefix": {
      "candidates": { "kind": "metadata_key", "target": "choice", "key": "skill" },
      "inputs": [
        { "name": "skill", "source": { "kind": "candidate_metadata", "key": "skill" }, "type": "int" },
        { "name": "tags", "source": { "kind": "candidate_metadata", "key": "tag", "occurrence": "all" }, "type": "symbol" }
      ],
      "queries": {
        "current": { "function": "missing", "args": [{ "input": "skill" }] }
      },
      "outputs": {
        "prefix": {
          "target": "candidate",
          "kind": "badge",
          "slot": "prefix",
          "label": {
            "template_id": "skill_check_prefix",
            "source_text": "[{skill} {current}]",
            "args": {
              "skill": { "source": { "input": "skill" }, "type": "string" }
            }
          },
          "fields": {
            "current": { "source": { "kind": "query_result", "name": "current" }, "type": "int" }
          }
        }
      }
    }
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
            "RECITE_SCHEMA001",
            "RECITE_SCHEMA004",
            "RECITE_SCHEMA001",
            "RECITE_SCHEMA001",
            "RECITE_SCHEMA004"
        ]
    );
}

#[test]
fn manifest_loader_rejects_invalid_nested_array_type_references() {
    let report = load_schema_manifest_str(
        "fixtures/schema/invalid/array_type_reference.json",
        include_str!("../../../../../fixtures/schema/invalid/array_type_reference.json"),
    );
    assert!(report.schema.is_none());
    assert_eq!(diagnostic_codes(&report), ["RECITE_SCHEMA004"]);

    let too_deep = "array:".repeat(9) + "string";
    let report = load_schema_manifest_str(
        "fixtures/schema/invalid/array_type_reference_depth.json",
        &format!(
            r#"{{"schema_version":1,"metadata":{{"bad":{{"targets":["line"],"type":"{too_deep}"}}}}}}"#
        ),
    );
    assert!(report.schema.is_none());
    assert_eq!(diagnostic_codes(&report), ["RECITE_SCHEMA004"]);
}

#[test]
fn manifest_loader_rejects_ambiguous_projection_and_domain_shapes() {
    for (name, source) in [
        (
            "mixed_input_query_result.json",
            r#"{
  "schema_version": 1,
  "presentation_projectors": { "p": {
    "candidates": { "kind": "candidate_project" },
    "queries": { "q": { "function": "f", "args": [{ "input": "x", "query_result": "y" }] } }
  } }
}"#,
        ),
        (
            "extra_occurrence_field.json",
            r#"{
  "schema_version": 1,
  "metadata": { "tag": { "targets": ["line"], "type": "string" } },
  "presentation_projectors": { "p": {
    "candidates": { "kind": "candidate_project" },
    "inputs": [{ "name": "x", "source": { "kind": "candidate_metadata", "key": "tag", "occurrence": { "index": 0, "extra": true } }, "type": "string" }]
  } }
}"#,
        ),
        (
            "wrong_kind_domain_fields.json",
            r#"{
  "schema_version": 1,
  "metadata_domains": { "bad": { "kind": "flat", "values": ["x"], "selector": "field:speaker" } }
}"#,
        ),
        (
            "empty_context_origins_on_flat_domain.json",
            r#"{
  "schema_version": 1,
  "metadata_domains": { "bad": { "kind": "flat", "values": ["x"], "context_origins": {} } }
}"#,
        ),
    ] {
        let report = load_schema_manifest_str(name, source);
        assert!(report.schema.is_none(), "{name} should be rejected");
        assert!(
            !report.diagnostics.is_empty(),
            "{name} should report a diagnostic"
        );
        crate::assert_recordable_diagnostics(&report);
    }
}

#[test]
fn manifest_loader_requires_explicit_generated_context_policy() {
    let report = load_schema_manifest_str(
        "fixtures/schema/invalid/contextual_missing_context.json",
        include_str!("../../../../../fixtures/schema/invalid/contextual_missing_context.json"),
    );
    assert!(report.schema.is_none());
    assert_eq!(diagnostic_codes(&report), ["RECITE_SCHEMA001"]);
    assert_eq!(
        report.diagnostics[0]
            .presentation
            .as_ref()
            .expect("missing-context presentation")
            .id()
            .as_str(),
        "diagnostic-schema-001-domain-missing-context"
    );
    assert_eq!(
        report.diagnostics[0]
            .presentation
            .as_ref()
            .expect("missing-context presentation")
            .arguments(),
        &std::collections::BTreeMap::from([(
            "domain".to_owned(),
            DiagnosticArgumentValue::String("tone_by_speaker".to_owned()),
        )])
    );
}

#[test]
fn manifest_loader_rejects_projection_all_with_wrong_array_inner_type() {
    let report = load_schema_manifest_str(
        "fixtures/schema/invalid/projection_all_wrong_array_inner.json",
        r#"{
  "schema_version": 1,
  "metadata": {
    "tag": {
      "targets": ["choice"],
      "type": "symbol",
      "repeatable": true
    }
  },
  "presentation_projectors": {
    "choice_tags": {
      "candidates": { "kind": "metadata_key", "target": "choice", "key": "tag" },
      "inputs": [
        { "name": "tags", "source": { "kind": "candidate_metadata", "key": "tag", "occurrence": "all" }, "type": "array:int" }
      ]
    }
  }
}"#,
    );

    assert!(report.schema.is_none());
    assert_eq!(diagnostic_codes(&report), ["RECITE_SCHEMA001"]);
}
