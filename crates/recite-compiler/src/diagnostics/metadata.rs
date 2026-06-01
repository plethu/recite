use recite_core::{Diagnostic, DiagnosticCode, SchemaTypeRef, SourceSpan};

use super::display_schema_type_ref;

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
    Diagnostic::error(
        UNKNOWN_METADATA_KEY,
        format!("unknown metadata key `{key}`"),
        span,
    )
    .with_help("declare the metadata key in the project schema manifest")
}

pub(crate) fn invalid_metadata_target(key: &str, target: &str, span: SourceSpan) -> Diagnostic {
    Diagnostic::error(
        INVALID_METADATA_TARGET,
        format!("metadata key `{key}` is not allowed on {target}"),
        span,
    )
    .with_help("move the metadata entry to an allowed target or update the project schema manifest")
}

pub(crate) fn duplicate_metadata_key(key: &str, span: SourceSpan) -> Diagnostic {
    Diagnostic::error(
        DUPLICATE_METADATA_KEY,
        format!("metadata key `{key}` is not repeatable"),
        span,
    )
    .with_help("remove the duplicate metadata entry or mark the key repeatable in the schema")
}

pub(crate) fn wrong_metadata_value_type(
    key: &str,
    expected: &SchemaTypeRef,
    actual: &str,
    span: SourceSpan,
) -> Diagnostic {
    Diagnostic::error(
        WRONG_METADATA_VALUE_TYPE,
        format!(
            "metadata key `{key}` expects {}, but got {actual}",
            display_schema_type_ref(expected),
        ),
        span,
    )
    .with_help("use a metadata value matching the project schema manifest")
}

pub(crate) fn invalid_metadata_value(
    key: &str,
    expected: &SchemaTypeRef,
    value: &str,
    span: SourceSpan,
) -> Diagnostic {
    Diagnostic::error(
        INVALID_METADATA_VALUE,
        format!(
            "metadata key `{key}` uses unknown {} value `{value}`",
            display_schema_type_ref(expected),
        ),
        span,
    )
    .with_help("use a value exported in the project schema manifest")
}

pub(crate) fn invalid_metadata_domain_value(
    key: &str,
    domain: &str,
    value: &str,
    span: SourceSpan,
) -> Diagnostic {
    Diagnostic::error(
        INVALID_METADATA_DOMAIN_VALUE,
        format!("metadata key `{key}` uses value `{value}` outside metadata domain `{domain}`"),
        span,
    )
    .with_help("use a symbol value exported in the metadata domain snapshot")
}

pub(crate) fn missing_metadata_domain_context(
    key: &str,
    domain: &str,
    selector: &str,
    span: SourceSpan,
) -> Diagnostic {
    Diagnostic::error(
        MISSING_METADATA_DOMAIN_CONTEXT,
        format!(
            "metadata key `{key}` cannot resolve selector `{selector}` for metadata domain `{domain}`"
        ),
        span,
    )
    .with_help("provide the selector context or update the domain missing-context policy")
}

pub(crate) fn malformed_metadata_domain_context(
    key: &str,
    selector: &str,
    span: SourceSpan,
) -> Diagnostic {
    Diagnostic::error(
        MALFORMED_METADATA_DOMAIN_CONTEXT,
        format!("metadata key `{key}` has ambiguous or non-symbol selector `{selector}`"),
        span,
    )
    .with_help("metadata domain selectors require exactly one scalar symbol context value")
}
