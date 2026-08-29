use recite_core::{Diagnostic, DiagnosticCode, SourceSpan};

use super::{compiler_diagnostic, diagnostic_contract, related_presentation, string_argument};

const UNKNOWN_MARKUP_TAG: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE022");
const UNBALANCED_MARKUP_TAG: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE023");
const MISSING_MARKUP_CLOSING_TAG: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE024");
const INVALID_MARKUP_NESTING: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE025");

pub(crate) use recite_core::MarkupUnbalancedKind as UnbalancedMarkupKind;

pub(crate) fn unknown_markup_tag(tag: &str, span: SourceSpan) -> Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&UNKNOWN_MARKUP_TAG, "diagnostic-validate-022"),
        format!("unknown inline markup tag `{tag}`"),
        span,
        vec![("tag".to_owned(), string_argument(tag))],
    )
    .with_help_presentation(super::auxiliary_presentation(
        "diagnostic-validate-022-help",
        [],
    ))
}

pub(crate) fn unbalanced_markup_tag(
    tag: &str,
    span: SourceSpan,
    kind: UnbalancedMarkupKind,
    related_opening: Option<SourceSpan>,
) -> Diagnostic {
    let (presentation_id, message, arguments) = match &kind {
        UnbalancedMarkupKind::MissingClosingBracket => (
            "diagnostic-validate-023-bracket",
            "unbalanced inline markup tag `[`: missing closing bracket".to_owned(),
            Vec::new(),
        ),
        UnbalancedMarkupKind::Standalone => (
            "diagnostic-validate-023-standalone",
            format!(
                "unbalanced inline markup tag `{tag}`: standalone tag does not use a closing tag"
            ),
            vec![("tag".to_owned(), string_argument(tag))],
        ),
        UnbalancedMarkupKind::NoOpening => (
            "diagnostic-validate-023-no-opening",
            format!(
                "unbalanced inline markup tag `{tag}`: closing tag has no matching opening tag"
            ),
            vec![("tag".to_owned(), string_argument(tag))],
        ),
        UnbalancedMarkupKind::Mismatch { expected } => (
            "diagnostic-validate-023-mismatch",
            format!(
                "unbalanced inline markup tag `{tag}`: expected closing tag for `{expected}` first"
            ),
            vec![
                ("tag".to_owned(), string_argument(tag)),
                ("expected_tag".to_owned(), string_argument(expected)),
            ],
        ),
    };
    let mut diagnostic = compiler_diagnostic(
        diagnostic_contract(&UNBALANCED_MARKUP_TAG, presentation_id),
        message,
        span,
        arguments,
    )
    .with_help_presentation(super::auxiliary_presentation(
        "diagnostic-validate-023-help",
        [],
    ));
    if let Some(opening) = related_opening {
        diagnostic = diagnostic.with_related_presentations([related_presentation(
            opening,
            "diagnostic-validate-023-related",
            [],
        )]);
    }
    diagnostic
}

pub(crate) fn missing_markup_closing_tag(tag: &str, span: SourceSpan) -> Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&MISSING_MARKUP_CLOSING_TAG, "diagnostic-validate-024"),
        format!("inline markup tag `{tag}` requires a closing tag"),
        span,
        vec![("tag".to_owned(), string_argument(tag))],
    )
    .with_help_presentation(super::auxiliary_presentation(
        "diagnostic-validate-024-help",
        vec![("tag".to_owned(), string_argument(tag))],
    ))
}

pub(crate) fn invalid_markup_nesting(
    parent: &str,
    child: &str,
    child_span: SourceSpan,
    parent_span: SourceSpan,
) -> Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&INVALID_MARKUP_NESTING, "diagnostic-validate-025"),
        format!("inline markup tag `{parent}` cannot contain nested tag `{child}`"),
        child_span,
        vec![
            ("parent".to_owned(), string_argument(parent)),
            ("child".to_owned(), string_argument(child)),
        ],
    )
    .with_related_presentations([related_presentation(
        parent_span,
        "diagnostic-validate-025-related",
        [],
    )])
    .with_help_presentation(super::auxiliary_presentation(
        "diagnostic-validate-025-help",
        vec![
            ("parent".to_owned(), string_argument(parent)),
            ("child".to_owned(), string_argument(child)),
        ],
    ))
}
