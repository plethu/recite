#[path = "schema_source_toml/contract.rs"]
mod schema_source_contract;
#[path = "schema_source_toml/diagnostic_spans.rs"]
mod schema_source_diagnostic_spans;
#[path = "schema_source_toml/support.rs"]
mod schema_source_diagnostic_support;
#[path = "schema_source_toml/diagnostics.rs"]
mod schema_source_diagnostics;
#[path = "schema_source_toml/edit_plans.rs"]
mod schema_source_edit_plans;
#[path = "schema_source_toml/edits.rs"]
mod schema_source_edits;
#[path = "schema_source_toml/freshness.rs"]
mod schema_source_freshness;
#[path = "schema_source_toml/numeric.rs"]
mod schema_source_numeric;
#[path = "schema_source_toml/numeric_projection.rs"]
mod schema_source_numeric_projection;
#[path = "schema_source_toml/structured_diagnostics.rs"]
mod schema_source_structured_diagnostics;

pub(crate) fn assert_recordable_diagnostics(report: &recite_core::SchemaSourceLoadReport) {
    for diagnostic in &report.diagnostics {
        assert!(
            diagnostic.presentation.is_some(),
            "schema source diagnostic must have a primary presentation: {diagnostic:?}"
        );
        assert!(
            diagnostic.related.is_empty(),
            "schema source diagnostic must not retain legacy related spans: {diagnostic:?}"
        );
        assert!(
            diagnostic.help.is_none(),
            "schema source diagnostic must not retain legacy help text: {diagnostic:?}"
        );
        assert!(
            diagnostic.record().is_ok(),
            "schema source diagnostic must be recordable"
        );
    }
}
