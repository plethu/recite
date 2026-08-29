use super::{assert_structured_diagnostic, string};
use recite_core::{DiagnosticArgumentValue, load_schema_manifest_str};

#[test]
fn manifest_parameter_diagnostics_have_exact_presentations() {
    let report = load_schema_manifest_str(
        "parameters.json",
        r#"{
  "schema_version": 1,
  "conditions": {
    "check": {
      "params": [
        { "name": "target", "type": "symbol" },
        { "name": "target", "type": "not-a-type" }
      ]
    }
  }
}"#,
    );
    assert!(report.schema.is_none());
    assert_eq!(report.diagnostics.len(), 3);
    assert_eq!(
        report
            .diagnostics
            .iter()
            .map(|diagnostic| diagnostic
                .presentation
                .as_ref()
                .expect("presentation")
                .id()
                .as_str())
            .collect::<Vec<_>>(),
        [
            "diagnostic-schema-004-parameter-special-type",
            "diagnostic-schema-003-parameter",
            "diagnostic-schema-004-invalid-parameter-type",
        ]
    );
    assert_primary_span(&report.diagnostics[0], "parameters.json", 6, 37, (6, 45));
    assert_primary_span(&report.diagnostics[1], "parameters.json", 7, 19, (7, 27));
    assert_primary_span(&report.diagnostics[2], "parameters.json", 7, 37, (7, 49));
    assert_structured_at(
        &report,
        0,
        "diagnostic-schema-004-parameter-special-type",
        &[
            ("owner", string("condition 'check'")),
            ("parameter", string("target")),
            ("type_ref", string("symbol")),
        ],
    );
    assert_structured_at(
        &report,
        1,
        "diagnostic-schema-003-parameter",
        &[
            ("owner", string("condition 'check'")),
            ("parameter", string("target")),
        ],
    );
    assert_structured_at(
        &report,
        2,
        "diagnostic-schema-004-invalid-parameter-type",
        &[
            ("parameter", string("target")),
            ("type_ref", string("not-a-type")),
        ],
    );
}

fn assert_primary_span(
    diagnostic: &recite_core::Diagnostic,
    file: &str,
    start_line: u32,
    start_column: u32,
    end: (u32, u32),
) {
    assert_eq!(diagnostic.span.file, file);
    assert_eq!(
        (
            diagnostic.span.start.line(),
            diagnostic.span.start.column(),
            diagnostic
                .span
                .end
                .map(|position| (position.line(), position.column()))
        ),
        (start_line, start_column, Some(end))
    );
}

fn assert_structured_at(
    report: &recite_core::SchemaLoadReport,
    index: usize,
    presentation_id: &str,
    arguments: &[(&str, DiagnosticArgumentValue)],
) {
    let diagnostic = report
        .diagnostics
        .get(index)
        .unwrap_or_else(|| panic!("missing diagnostic at index {index}"));
    assert_structured_diagnostic(diagnostic, presentation_id, arguments);
}
