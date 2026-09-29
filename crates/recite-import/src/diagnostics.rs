use recite_core::{
    Diagnostic, DiagnosticArgumentValue, DiagnosticCode, DiagnosticSeverity, SourcePosition,
    SourceSpan, contracts_for_code,
};

use crate::{Action, ImportError, ImportItem, Location, Provenance};

pub(super) const INVALID: DiagnosticCode = DiagnosticCode::new_static("RECITE_IMPORT001");
pub(super) const UNSUPPORTED: DiagnosticCode = DiagnosticCode::new_static("RECITE_IMPORT002");
pub(super) const LOSS: DiagnosticCode = DiagnosticCode::new_static("RECITE_IMPORT003");

pub(super) fn item(
    code: DiagnosticCode,
    provenance: Provenance,
    construct: &str,
    detail: impl Into<String>,
) -> Result<ImportItem, ImportError> {
    let detail = detail.into();
    let (severity, action, follow_up) = if code == INVALID {
        (
            DiagnosticSeverity::Error,
            Action::Rejected,
            "Correct the input or mapping and inspect again.",
        )
    } else if code == LOSS {
        (
            DiagnosticSeverity::Warning,
            Action::ConvertedWithLoss,
            "Review the generated source against the original behavior.",
        )
    } else {
        (
            DiagnosticSeverity::Warning,
            Action::Skipped,
            "Migrate this construct manually before using the generated source.",
        )
    };
    let span = match &provenance.location {
        Location::Text { span } => span.clone(),
        _ => SourceSpan::point(&provenance.file, SourcePosition::new(1, 1)?),
    };
    let contract = contracts_for_code(&code)
        .next()
        .ok_or(ImportError::MissingContract("RECITE_IMPORT"))?;
    let diagnostic = Diagnostic::from_contract(
        severity,
        contract,
        detail.clone(),
        span,
        [("detail", DiagnosticArgumentValue::String(detail))],
    )?
    .record()?;
    Ok(ImportItem {
        diagnostic,
        provenance,
        construct: construct.to_owned(),
        action,
        follow_up: follow_up.to_owned(),
    })
}
