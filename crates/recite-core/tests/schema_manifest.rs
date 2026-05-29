#![cfg(test)]

#[path = "schema_manifest/mod.rs"]
mod manifest_tests;

pub(crate) fn diagnostic_codes(report: &recite_core::SchemaLoadReport) -> Vec<&str> {
    report
        .diagnostics
        .iter()
        .map(|diagnostic| diagnostic.code.as_str())
        .collect()
}
