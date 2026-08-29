#![cfg(test)]

#[path = "schema_manifest/mod.rs"]
mod manifest_tests;

pub(crate) fn diagnostic_codes(report: &recite_core::SchemaLoadReport) -> Vec<&str> {
    assert_recordable_diagnostics(report);
    report
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect()
}

pub(crate) fn assert_recordable_diagnostics(report: &recite_core::SchemaLoadReport) {
    for diagnostic in &report.diagnostics {
        assert!(
            diagnostic.presentation.is_some(),
            "schema diagnostic must have a primary presentation: {diagnostic:?}"
        );
        assert!(
            diagnostic.related.is_empty(),
            "schema diagnostic must not retain legacy related spans: {diagnostic:?}"
        );
        assert!(
            diagnostic.help.is_none(),
            "schema diagnostic must not retain legacy help text: {diagnostic:?}"
        );
        diagnostic
            .record()
            .expect("schema diagnostic must be recordable");
    }
}
