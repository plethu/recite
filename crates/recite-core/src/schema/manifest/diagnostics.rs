use crate::{Diagnostic, DiagnosticCode, DiagnosticSeverity, SourceSpan};

pub(crate) const MALFORMED_SHAPE: &str = "RECITE_SCHEMA001";
pub(crate) const UNSUPPORTED_VERSION: &str = "RECITE_SCHEMA002";
pub(crate) const DUPLICATE_DEFINITION: &str = "RECITE_SCHEMA003";
pub(crate) const INVALID_TYPE_REFERENCE: &str = "RECITE_SCHEMA004";

pub(crate) fn diagnostic(code: &str, message: impl Into<String>, span: SourceSpan) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::new(code).expect("schema diagnostic codes are static and namespaced"),
        DiagnosticSeverity::Error,
        message,
        span,
    )
}
