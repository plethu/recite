use recite_core::{Diagnostic, DiagnosticCode, SourceSpan};

use super::{compiler_diagnostic, diagnostic_contract, related_presentation, string_argument};

const DUPLICATE_CASE: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE052");
const CASE_AFTER_WILDCARD: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE053");

/// A second `:case <variant>` arm for a variant an earlier arm covers.
pub(crate) fn duplicate_match_case(
    variant: &str,
    span: SourceSpan,
    first_span: SourceSpan,
) -> Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&DUPLICATE_CASE, "diagnostic-validate-052"),
        format!("duplicate `:case {variant}` arm is never selected"),
        span,
        vec![("pattern".to_owned(), string_argument(variant))],
    )
    .with_related_presentations([related_presentation(
        first_span,
        "diagnostic-validate-052-related",
        [],
    )])
    .with_help_presentation(super::auxiliary_presentation(
        "diagnostic-validate-052-help",
        [],
    ))
}

/// An arm after the wildcard arm, including a second wildcard.
pub(crate) fn match_case_after_wildcard(
    pattern: &str,
    span: SourceSpan,
    wildcard_span: SourceSpan,
) -> Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&CASE_AFTER_WILDCARD, "diagnostic-validate-053"),
        format!("`:case {pattern}` follows the wildcard arm; `:case _` must be the last arm"),
        span,
        vec![("pattern".to_owned(), string_argument(pattern))],
    )
    .with_related_presentations([related_presentation(
        wildcard_span,
        "diagnostic-validate-053-related",
        [],
    )])
    .with_help_presentation(super::auxiliary_presentation(
        "diagnostic-validate-053-help",
        [],
    ))
}
