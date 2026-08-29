use recite_core::{Diagnostic, DiagnosticCode, SourceSpan};

use super::super::{compiler_diagnostic, diagnostic_contract, string_argument};

const INVALID_INTERPOLATION: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE045");

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum InterpolationError {
    Unterminated,
    UnescapedClosingBrace,
    InvalidName(String),
    Duplicate(String),
    Unused(String),
    Unbound(String),
}

pub(crate) fn invalid_interpolation(span: SourceSpan, error: InterpolationError) -> Diagnostic {
    let (presentation_id, message, arguments) = match error {
        InterpolationError::Unterminated => (
            "diagnostic-validate-045-unterminated",
            "invalid interpolation binding: unterminated placeholder".to_owned(),
            Vec::new(),
        ),
        InterpolationError::UnescapedClosingBrace => (
            "diagnostic-validate-045-unescaped",
            "invalid interpolation binding: unescaped closing brace".to_owned(),
            Vec::new(),
        ),
        InterpolationError::InvalidName(name) => (
            "diagnostic-validate-045-invalid-name",
            format!("invalid interpolation binding: invalid placeholder name '{name}'"),
            vec![("key".to_owned(), string_argument(name))],
        ),
        InterpolationError::Duplicate(name) => (
            "diagnostic-validate-045-duplicate",
            format!(
                "invalid interpolation binding: placeholder `{name}` is declared more than once"
            ),
            vec![("key".to_owned(), string_argument(name))],
        ),
        InterpolationError::Unused(name) => (
            "diagnostic-validate-045-unused",
            format!("invalid interpolation binding: binding `{name}` is not used in the text"),
            vec![("key".to_owned(), string_argument(name))],
        ),
        InterpolationError::Unbound(name) => (
            "diagnostic-validate-045-unbound",
            format!("invalid interpolation binding: placeholder `{name}` has no binding"),
            vec![("key".to_owned(), string_argument(name))],
        ),
    };
    compiler_diagnostic(
        diagnostic_contract(&INVALID_INTERPOLATION, presentation_id),
        message,
        span,
        arguments,
    )
}
