use recite_core::{Diagnostic, DiagnosticCode, DiagnosticSeverity, SourceSpan};

pub(crate) fn diagnostic(code: &str, message: &str, span: SourceSpan) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::new(code).expect("parser diagnostic codes are static and namespaced"),
        DiagnosticSeverity::Error,
        message,
        span,
    )
}
