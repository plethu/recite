use std::path::Path;

use recite_core::{
    Diagnostic, DiagnosticArgumentValue, DiagnosticCode, DiagnosticPresentationId, SourcePosition,
    SourceSpan, contract_for,
};

use super::SchemaKind;

const SCHEMA_LOAD_ERROR: DiagnosticCode = DiagnosticCode::new_static("RECITE_SCHEMA001");

pub(super) fn schema_kind(path: &Path) -> SchemaKind {
    match path
        .extension()
        .and_then(|extension| extension.to_str())
        .map(str::to_ascii_lowercase)
        .as_deref()
    {
        Some("toml") => SchemaKind::Toml,
        Some("json") => SchemaKind::Json,
        _ => SchemaKind::Unknown,
    }
}

#[allow(
    clippy::expect_used,
    reason = "the schema read contract is a static first-party registry invariant"
)]
pub(super) fn schema_io_diagnostic(file: String, error: &std::io::Error) -> Vec<Diagnostic> {
    let Ok(start) = SourcePosition::new(1, 1) else {
        return Vec::new();
    };
    let presentation_id = DiagnosticPresentationId::new_static("diagnostic-schema-001-read");
    let contract = contract_for(&SCHEMA_LOAD_ERROR, &presentation_id)
        .expect("schema read diagnostic contract is registered");
    let diagnostic = Diagnostic::error_from_contract(
        contract,
        format!("failed to read schema manifest: {error}"),
        SourceSpan::new(file, start, None),
        [("detail", DiagnosticArgumentValue::String(error.to_string()))],
    )
    .expect("schema read diagnostic arguments match their contract");
    vec![diagnostic]
}

pub(super) fn schema_unavailable_diagnostic(file: String) -> Vec<Diagnostic> {
    let Ok(start) = SourcePosition::new(1, 1) else {
        return Vec::new();
    };
    let presentation_id = DiagnosticPresentationId::new_static("diagnostic-schema-001-read");
    let Some(contract) = contract_for(&SCHEMA_LOAD_ERROR, &presentation_id) else {
        return Vec::new();
    };
    Diagnostic::error_from_contract(
        contract,
        "schema format is unavailable for this file extension",
        SourceSpan::new(file, start, None),
        [(
            "detail",
            DiagnosticArgumentValue::String("expected .toml or .json".to_owned()),
        )],
    )
    .ok()
    .into_iter()
    .collect()
}
