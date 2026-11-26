use recite_core::{Diagnostic, DiagnosticCode, SourceSpan};

use super::{compiler_diagnostic, diagnostic_contract, related_presentation, string_argument};

const DUPLICATE_CASE: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE052");
const CASE_AFTER_WILDCARD: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE053");
const UNKNOWN_VARIANT: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE054");
const NON_EXHAUSTIVE: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE055");

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

/// A `:case` variant the scrutinee's schema enum does not declare.
pub(crate) fn unknown_match_variant(
    variant: &str,
    enum_name: &str,
    span: SourceSpan,
    suggestion: Option<&str>,
) -> Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&UNKNOWN_VARIANT, "diagnostic-validate-054"),
        format!("`:case {variant}` is not a variant of enum `{enum_name}`"),
        span,
        vec![
            ("pattern".to_owned(), string_argument(variant)),
            ("enum".to_owned(), string_argument(enum_name)),
        ],
    )
    .with_help_presentation(match suggestion {
        Some(suggestion) => super::auxiliary_presentation(
            "diagnostic-validate-054-help-suggestion",
            [("suggestion".to_owned(), string_argument(suggestion))],
        ),
        None => super::auxiliary_presentation("diagnostic-validate-054-help", []),
    })
}

/// A `:match` without `:case _` that leaves declared variants without an arm.
pub(crate) fn non_exhaustive_match(
    enum_name: &str,
    missing: &[&str],
    span: SourceSpan,
) -> Diagnostic {
    let missing = missing
        .iter()
        .map(|variant| format!("`{variant}`"))
        .collect::<Vec<_>>()
        .join(", ");
    compiler_diagnostic(
        diagnostic_contract(&NON_EXHAUSTIVE, "diagnostic-validate-055"),
        format!("match on enum `{enum_name}` has no arm for {missing}"),
        span,
        vec![
            ("enum".to_owned(), string_argument(enum_name)),
            ("missing".to_owned(), string_argument(missing)),
        ],
    )
    .with_help_presentation(super::auxiliary_presentation(
        "diagnostic-validate-055-help",
        [],
    ))
}
