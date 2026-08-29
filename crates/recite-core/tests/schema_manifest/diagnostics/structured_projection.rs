use recite_core::{
    DiagnosticArgumentValue, DiagnosticCode, DiagnosticPresentationId, DiagnosticSeverity,
    contract_for, load_schema_manifest_str,
};
use std::collections::BTreeMap;

#[test]
fn projection_contract_family_is_registered() {
    for (code, id) in [
        (
            "RECITE_SCHEMA001",
            "diagnostic-schema-001-projection-candidate-source",
        ),
        (
            "RECITE_SCHEMA003",
            "diagnostic-schema-003-projection-output",
        ),
        (
            "RECITE_SCHEMA004",
            "diagnostic-schema-004-unknown-query-function",
        ),
        (
            "RECITE_SCHEMA001",
            "diagnostic-schema-001-label-placeholder-unterminated",
        ),
        (
            "RECITE_SCHEMA001",
            "diagnostic-schema-001-label-placeholder-invalid-name",
        ),
        (
            "RECITE_SCHEMA001",
            "diagnostic-schema-001-label-placeholder-unescaped-closing-brace",
        ),
    ] {
        assert!(
            contract_for(
                &DiagnosticCode::new_static(code),
                &DiagnosticPresentationId::new_static(id),
            )
            .is_some(),
            "missing projection contract {id}"
        );
    }
}

#[test]
fn projection_diagnostics_use_typed_presentations_and_are_recordable() {
    let report = load_schema_manifest_str(
        "projection-structured.json",
        r#"{
  "schema_version": 1,
  "projection_queries": {
    "query": {
      "returns": "string",
      "max_calls_per_event": 0
    }
  },
  "presentation_projectors": {
    "projector": {
      "candidates": { "kind": "runtime_event", "event": "dialogue" },
      "queries": {
        "missing": { "function": "unknown", "args": [] }
      },
      "outputs": {
        "output": {
          "target": "invalid",
          "kind": "badge",
          "slot": "prefix",
          "label": {
            "template_id": "label",
            "source_text": "{",
            "args": {}
          },
          "fields": {}
        }
      }
    }
  }
}"#,
    );
    crate::assert_recordable_diagnostics(&report);

    let ids = report
        .diagnostics
        .iter()
        .map(|diagnostic| {
            diagnostic
                .record()
                .expect("projection diagnostic should be recordable");
            diagnostic
                .presentation
                .as_ref()
                .expect("projection diagnostic presentation")
                .id()
                .as_str()
        })
        .collect::<Vec<_>>();
    assert_eq!(
        ids,
        [
            "diagnostic-schema-001-query-max-calls",
            "diagnostic-schema-004-unknown-query-function",
            "diagnostic-schema-001-projection-output-target",
            "diagnostic-schema-001-label-placeholder-unterminated",
        ]
    );
}

#[test]
fn projection_type_reference_diagnostics_use_typed_presentations_and_quoted_spans() {
    let report = load_schema_manifest_str(
        "projection-type-contexts.json",
        r#"{
  "schema_version": 1,
  "projection_queries": {
    "lookup": {
      "returns": "bad-query-return"
    }
  },
  "presentation_projectors": {
    "cards": {
      "candidates": { "kind": "runtime_event", "event": "dialogue" },
      "inputs": [
        {
          "name": "event",
          "type": "bad-input",
          "source": { "kind": "event_kind" }
        }
      ],
      "queries": {},
      "outputs": {
        "badge": {
          "target": "candidate",
          "kind": "badge",
          "slot": "prefix",
          "label": {
            "template_id": "badge-label",
            "source_text": "{event}",
            "args": {
              "event": {
                "source": { "input": "event" },
                "type": "bad-label"
              }
            }
          },
          "fields": {
            "event": {
              "source": { "kind": "input", "name": "event" },
              "type": "bad-field"
            }
          }
        }
      }
    }
  }
}"#,
    );

    assert!(report.schema.is_none());
    assert_eq!(report.diagnostics.len(), 4);
    assert_projection_type_diagnostic(
        &report.diagnostics[0],
        "diagnostic-schema-004-invalid-query-return-type",
        &[("function", "lookup"), ("type_ref", "bad-query-return")],
        5,
        18,
        36,
    );
    assert_projection_type_diagnostic(
        &report.diagnostics[1],
        "diagnostic-schema-004-invalid-projection-input-type",
        &[
            ("projector", "cards"),
            ("input", "event"),
            ("type_ref", "bad-input"),
        ],
        14,
        19,
        30,
    );
    assert_projection_type_diagnostic(
        &report.diagnostics[2],
        "diagnostic-schema-004-invalid-projection-output-type",
        &[
            ("projector", "cards"),
            ("output", "badge"),
            ("binding", "event"),
            ("type_ref", "bad-label"),
        ],
        30,
        25,
        36,
    );
    assert_projection_type_diagnostic(
        &report.diagnostics[3],
        "diagnostic-schema-004-invalid-projection-output-type",
        &[
            ("projector", "cards"),
            ("output", "badge"),
            ("binding", "event"),
            ("type_ref", "bad-field"),
        ],
        37,
        23,
        34,
    );
}

fn assert_projection_type_diagnostic(
    diagnostic: &recite_core::Diagnostic,
    presentation_id: &str,
    arguments: &[(&str, &str)],
    line: u32,
    start_column: u32,
    end_column: u32,
) {
    assert_eq!(diagnostic.code.as_str(), "RECITE_SCHEMA004");
    assert_eq!(diagnostic.severity, DiagnosticSeverity::Error);
    assert!(diagnostic.related.is_empty());
    assert!(diagnostic.help.is_none());
    assert_eq!(diagnostic.span.file, "projection-type-contexts.json");
    assert_eq!(diagnostic.span.start.line(), line);
    assert_eq!(diagnostic.span.start.column(), start_column);
    assert_eq!(
        diagnostic
            .span
            .end
            .map(|position| (position.line(), position.column())),
        Some((line, end_column))
    );
    let expected = arguments
        .iter()
        .map(|(name, value)| {
            (
                (*name).to_owned(),
                DiagnosticArgumentValue::String((*value).to_owned()),
            )
        })
        .collect::<BTreeMap<_, _>>();
    assert_eq!(
        diagnostic
            .presentation
            .as_ref()
            .expect("structured presentation")
            .id()
            .as_str(),
        presentation_id
    );
    assert_eq!(
        diagnostic
            .presentation
            .as_ref()
            .expect("structured presentation")
            .arguments(),
        &expected
    );
    diagnostic
        .record()
        .expect("recordable structured diagnostic");
}
