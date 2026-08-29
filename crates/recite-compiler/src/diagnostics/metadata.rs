use recite_core::{Diagnostic, DiagnosticCode, MetadataTarget, SchemaTypeRef, SourceSpan};

use super::{compiler_diagnostic, diagnostic_contract, string_argument};

const UNKNOWN_METADATA_KEY: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE026");
const INVALID_METADATA_TARGET: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE027");
const DUPLICATE_METADATA_KEY: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE028");
const WRONG_METADATA_VALUE_TYPE: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE029");
const INVALID_METADATA_VALUE: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE030");
const INVALID_METADATA_DOMAIN_VALUE: DiagnosticCode =
    DiagnosticCode::new_static("RECITE_VALIDATE031");
const MISSING_METADATA_DOMAIN_CONTEXT: DiagnosticCode =
    DiagnosticCode::new_static("RECITE_VALIDATE032");
const MALFORMED_METADATA_DOMAIN_CONTEXT: DiagnosticCode =
    DiagnosticCode::new_static("RECITE_VALIDATE033");

pub(crate) fn unknown_metadata_key(key: &str, span: SourceSpan) -> Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&UNKNOWN_METADATA_KEY, "diagnostic-validate-026"),
        format!("unknown metadata key `{key}`"),
        span,
        vec![("key".to_owned(), string_argument(key))],
    )
    .with_help_presentation(super::auxiliary_presentation(
        "diagnostic-validate-026-help",
        [],
    ))
}

pub(crate) fn invalid_metadata_target(
    key: &str,
    target: MetadataTarget,
    span: SourceSpan,
) -> Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&INVALID_METADATA_TARGET, "diagnostic-validate-027"),
        format!(
            "metadata key `{key}` is not allowed on {}",
            target_token(target)
        ),
        span,
        vec![
            ("key".to_owned(), string_argument(key)),
            ("target".to_owned(), string_argument(target_token(target))),
        ],
    )
    .with_help_presentation(super::auxiliary_presentation(
        "diagnostic-validate-027-help",
        [],
    ))
}

pub(crate) fn duplicate_metadata_key(key: &str, span: SourceSpan) -> Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&DUPLICATE_METADATA_KEY, "diagnostic-validate-028"),
        format!("metadata key `{key}` is not repeatable"),
        span,
        vec![("key".to_owned(), string_argument(key))],
    )
    .with_help_presentation(super::auxiliary_presentation(
        "diagnostic-validate-028-help",
        [],
    ))
}

pub(crate) fn wrong_metadata_value_type(
    key: &str,
    expected: &SchemaTypeRef,
    actual: &str,
    span: SourceSpan,
) -> Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&WRONG_METADATA_VALUE_TYPE, "diagnostic-validate-029"),
        format!(
            "metadata key `{key}` expects {}, but got {actual}",
            display_schema_type_ref(expected),
        ),
        span,
        vec![
            ("key".to_owned(), string_argument(key)),
            (
                "expected".to_owned(),
                string_argument(display_schema_type_ref(expected)),
            ),
            ("actual".to_owned(), string_argument(actual)),
        ],
    )
    .with_help_presentation(super::auxiliary_presentation(
        "diagnostic-validate-029-help",
        [],
    ))
}

pub(crate) fn invalid_metadata_value(
    key: &str,
    expected: &SchemaTypeRef,
    value: &str,
    span: SourceSpan,
) -> Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&INVALID_METADATA_VALUE, "diagnostic-validate-030"),
        format!(
            "metadata key `{key}` uses unknown {} value `{value}`",
            display_schema_type_ref(expected),
        ),
        span,
        vec![
            ("key".to_owned(), string_argument(key)),
            (
                "expected".to_owned(),
                string_argument(display_schema_type_ref(expected)),
            ),
            ("value".to_owned(), string_argument(value)),
        ],
    )
    .with_help_presentation(super::auxiliary_presentation(
        "diagnostic-validate-030-help",
        [],
    ))
}

pub(crate) fn invalid_metadata_domain_value(
    key: &str,
    domain: &str,
    value: &str,
    span: SourceSpan,
) -> Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&INVALID_METADATA_DOMAIN_VALUE, "diagnostic-validate-031"),
        format!("metadata key `{key}` uses value `{value}` outside metadata domain `{domain}`"),
        span,
        vec![
            ("key".to_owned(), string_argument(key)),
            ("domain".to_owned(), string_argument(domain)),
            ("value".to_owned(), string_argument(value)),
        ],
    )
    .with_help_presentation(super::auxiliary_presentation(
        "diagnostic-validate-031-help",
        [],
    ))
}

pub(crate) fn missing_metadata_domain_context(
    key: &str,
    domain: &str,
    selector: &str,
    span: SourceSpan,
) -> Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&MISSING_METADATA_DOMAIN_CONTEXT, "diagnostic-validate-032"),
        format!(
            "metadata key `{key}` cannot resolve selector `{selector}` for metadata domain `{domain}`"
        ),
        span,
        vec![
            ("key".to_owned(), string_argument(key)),
            ("domain".to_owned(), string_argument(domain)),
            ("selector".to_owned(), string_argument(selector)),
        ],
    )
    .with_help_presentation(super::auxiliary_presentation("diagnostic-validate-032-help", []))
}

pub(crate) fn malformed_metadata_domain_context(
    key: &str,
    selector: &str,
    span: SourceSpan,
) -> Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(
            &MALFORMED_METADATA_DOMAIN_CONTEXT,
            "diagnostic-validate-033",
        ),
        format!("metadata key `{key}` has ambiguous or non-symbol selector `{selector}`"),
        span,
        vec![
            ("key".to_owned(), string_argument(key)),
            ("selector".to_owned(), string_argument(selector)),
        ],
    )
    .with_help_presentation(super::auxiliary_presentation(
        "diagnostic-validate-033-help",
        [],
    ))
}

fn target_token(target: MetadataTarget) -> &'static str {
    match target {
        MetadataTarget::Block => "block",
        MetadataTarget::Line => "line",
        MetadataTarget::Choice => "choice",
        MetadataTarget::Project => "project",
    }
}

fn display_schema_type_ref(type_ref: &SchemaTypeRef) -> String {
    super::display_schema_type_ref(type_ref)
}
