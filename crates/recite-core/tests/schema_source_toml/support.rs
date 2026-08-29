use std::collections::BTreeMap;

use recite_core::{Diagnostic, DiagnosticArgumentValue, SchemaSourceLoadReport};

pub(crate) fn string(value: &str) -> DiagnosticArgumentValue {
    DiagnosticArgumentValue::String(value.to_owned())
}

pub(crate) fn assert_presentation(
    diagnostic: &Diagnostic,
    presentation_id: &str,
    arguments: impl IntoIterator<Item = (&'static str, DiagnosticArgumentValue)>,
) {
    let expected = arguments
        .into_iter()
        .map(|(name, value)| (name.to_owned(), value))
        .collect::<BTreeMap<_, _>>();
    assert_presentation_map(diagnostic, presentation_id, &expected);
}

fn assert_presentation_map(
    diagnostic: &Diagnostic,
    presentation_id: &str,
    expected: &BTreeMap<String, DiagnosticArgumentValue>,
) {
    let Some(presentation) = diagnostic.presentation.as_ref() else {
        panic!("schema source diagnostic presentation");
    };
    assert_eq!(presentation.id().as_str(), presentation_id);
    assert_eq!(presentation.arguments(), expected);
    assert!(diagnostic.related.is_empty());
    assert!(diagnostic.help.is_none());
    assert!(
        diagnostic.record().is_ok(),
        "schema source diagnostic is recordable"
    );
}

pub(crate) fn assert_presentation_by_id<'a>(
    report: &'a SchemaSourceLoadReport,
    presentation_id: &str,
    arguments: impl IntoIterator<Item = (&'static str, DiagnosticArgumentValue)>,
) -> &'a Diagnostic {
    let expected = arguments
        .into_iter()
        .map(|(name, value)| (name.to_owned(), value))
        .collect::<BTreeMap<_, _>>();
    let diagnostic = report
        .diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic
                .presentation
                .as_ref()
                .is_some_and(|presentation| {
                    presentation.id().as_str() == presentation_id
                        && presentation.arguments() == &expected
                })
        })
        .unwrap_or_else(|| panic!("missing schema source diagnostic {presentation_id}"));
    assert_presentation_map(diagnostic, presentation_id, &expected);
    diagnostic
}

pub(crate) fn assert_empty_value_field(report: &SchemaSourceLoadReport, field: &str, line: u32) {
    let diagnostic = report
        .diagnostics
        .iter()
        .find(|diagnostic| {
            diagnostic
                .presentation
                .as_ref()
                .is_some_and(|presentation| {
                    presentation.id().as_str() == "diagnostic-schema-001-empty-value"
                })
                && matches!(
                    diagnostic
                        .presentation
                        .as_ref()
                        .and_then(|presentation| presentation.arguments().get("field")),
                    Some(DiagnosticArgumentValue::String(value)) if value == field
                )
        })
        .unwrap_or_else(|| panic!("missing empty-value diagnostic for {field}"));
    assert_presentation(
        diagnostic,
        "diagnostic-schema-001-empty-value",
        [("field", string(field))],
    );
    assert_eq!(diagnostic.span.start.line(), line, "{field}");
}
