use recite_core::{Diagnostic, DiagnosticCode, SourceSpan};

use super::super::{compiler_diagnostic, diagnostic_contract};

const INVALID_PLURAL_LINE: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE046");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum PluralError {
    Newline,
    MissingCount,
    CountType,
}

pub(crate) fn invalid_plural_line(span: SourceSpan, error: PluralError) -> Diagnostic {
    let (presentation_id, message) = match error {
        PluralError::Newline => (
            "diagnostic-validate-046-newline",
            "invalid plural line: plural lines must contain exactly one singular and one plural body line",
        ),
        PluralError::MissingCount => (
            "diagnostic-validate-046-missing-count",
            "invalid plural line: plural lines require `bind=(count:int=$value)`",
        ),
        PluralError::CountType => (
            "diagnostic-validate-046-count-type",
            "invalid plural line: the `count` binding must have type `int`",
        ),
    };
    compiler_diagnostic(
        diagnostic_contract(&INVALID_PLURAL_LINE, presentation_id),
        message,
        span,
        [],
    )
}
