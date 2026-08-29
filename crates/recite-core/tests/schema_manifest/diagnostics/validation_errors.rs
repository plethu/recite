use std::collections::BTreeMap;

use recite_core::{Diagnostic, DiagnosticArgumentValue, load_schema_manifest_str};

use crate::diagnostic_codes;

fn string(value: &str) -> DiagnosticArgumentValue {
    DiagnosticArgumentValue::String(value.to_owned())
}

fn assert_presentation(
    diagnostic: &Diagnostic,
    presentation_id: &str,
    arguments: impl IntoIterator<Item = (&'static str, DiagnosticArgumentValue)>,
) {
    let presentation = diagnostic
        .presentation
        .as_ref()
        .expect("schema diagnostic presentation");
    assert_eq!(presentation.id().as_str(), presentation_id);
    assert_eq!(
        presentation.arguments(),
        &arguments
            .into_iter()
            .map(|(name, value)| (name.to_owned(), value))
            .collect::<BTreeMap<_, _>>()
    );
}

#[test]
fn duplicate_definitions_and_values_report_stable_diagnostics() {
    let duplicate_definitions = load_schema_manifest_str(
        "fixtures/schema/invalid/duplicate_definitions.json",
        include_str!("../../../../../fixtures/schema/invalid/duplicate_definitions.json"),
    );
    assert!(duplicate_definitions.schema.is_none());
    assert_eq!(
        diagnostic_codes(&duplicate_definitions),
        ["RECITE_SCHEMA003"]
    );
    assert_eq!(duplicate_definitions.diagnostics[0].span.start.line(), 8);

    let duplicate_values = load_schema_manifest_str(
        "fixtures/schema/invalid/duplicate_values.json",
        include_str!("../../../../../fixtures/schema/invalid/duplicate_values.json"),
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
        include_str!("../../../../../fixtures/schema/invalid/invalid_type_references.json"),
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
    assert_presentation(
        &report.diagnostics[0],
        "diagnostic-schema-004-unknown-registry",
        [
            (
                "owner",
                string("condition 'thread_stage' parameter 'thread_id'"),
            ),
            ("name", string("thread")),
        ],
    );
    assert_eq!(report.diagnostics[0].span.start.line(), 5);
    assert_eq!(report.diagnostics[1].span.start.line(), 6);
    assert_eq!(report.diagnostics[2].span.start.line(), 12);
    assert_eq!(report.diagnostics[3].span.start.line(), 18);
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

#[test]
fn malformed_availability_reason_definitions_report_schema_diagnostics() {
    let report = load_schema_manifest_str(
        "fixtures/schema/invalid/availability_reason_definitions.json",
        r#"{
  "schema_version": 1,
  "availability_reasons": {
    "bad name": {
      "template": "{subject}",
      "params": [{ "name": "subject", "type": "speaker" }]
    },
    "unused_param": {
      "template": "No placeholders.",
      "params": [{ "name": "subject", "type": "speaker" }]
    },
    "unknown_placeholder": {
      "template": "{subject} {target}",
      "params": [{ "name": "subject", "type": "speaker" }]
    },
    "symbol_param": {
      "template": "{subject}",
      "params": [{ "name": "subject", "type": "symbol" }]
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
            "RECITE_SCHEMA004"
        ]
    );
}

#[test]
fn malformed_condition_availability_reason_mappings_report_schema_diagnostics() {
    let report = load_schema_manifest_str(
        "fixtures/schema/invalid/availability_reason_mappings.json",
        r#"{
  "schema_version": 1,
  "types": {
    "stage": { "kind": "enum", "values": ["intro"] }
  },
  "speakers": {
    "hazel": {}
  },
  "conditions": {
    "stage_is": {
      "returns": "enum:stage",
      "availability_reason": {
        "reason": "need_trust",
        "args": { "subject": "hazel" }
      }
    },
    "trust_gte": {
      "params": [{ "name": "actor", "type": "speaker" }],
      "availability_reason": {
        "reason": "need_trust",
        "args": {
          "subject": "$missing",
          "threshold": "high",
          "extra": true
        }
      }
    },
    "unknown_reason": {
      "availability_reason": {
        "reason": "missing_reason",
        "args": {}
      }
    }
  },
  "availability_reasons": {
    "need_trust": {
      "template": "{subject} {threshold}",
      "params": [
        { "name": "subject", "type": "speaker" },
        { "name": "threshold", "type": "int" }
      ]
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
            "RECITE_SCHEMA004",
            "RECITE_SCHEMA001",
            "RECITE_SCHEMA001",
            "RECITE_SCHEMA001",
            "RECITE_SCHEMA001",
            "RECITE_SCHEMA004"
        ]
    );
    let stage_is = report
        .diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic
                .presentation
                .as_ref()
                .is_some_and(|presentation| {
                    presentation.id().as_str()
                        == "diagnostic-schema-001-availability-missing-reason-arg"
                        && presentation.arguments().get("condition") == Some(&string("stage_is"))
                        && presentation.arguments().get("argument") == Some(&string("threshold"))
                })
        })
        .expect("missing stage_is threshold diagnostic");
    assert_presentation(
        stage_is,
        "diagnostic-schema-001-availability-missing-reason-arg",
        [
            ("condition", string("stage_is")),
            ("argument", string("threshold")),
        ],
    );
    assert_eq!(stage_is.span.start.line(), 13);
    let trust_gte = report
        .diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic
                .presentation
                .as_ref()
                .is_some_and(|presentation| {
                    presentation.id().as_str() == "diagnostic-schema-004-unknown-condition-param"
                        && presentation.arguments().get("condition") == Some(&string("trust_gte"))
                        && presentation.arguments().get("condition_param")
                            == Some(&string("missing"))
                })
        })
        .expect("missing trust_gte condition parameter diagnostic");
    assert_presentation(
        trust_gte,
        "diagnostic-schema-004-unknown-condition-param",
        [
            ("condition", string("trust_gte")),
            ("condition_param", string("missing")),
        ],
    );
    assert_eq!(trust_gte.span.start.line(), 22);
}
