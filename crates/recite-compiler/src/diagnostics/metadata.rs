use recite_core::{Diagnostic, DiagnosticCode, SchemaTypeRef, SourceSpan};

use super::display_schema_type_ref;

const UNKNOWN_METADATA_KEY: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE026");
const INVALID_METADATA_TARGET: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE027");
const DUPLICATE_METADATA_KEY: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE028");
const WRONG_METADATA_VALUE_TYPE: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE029");
const INVALID_METADATA_VALUE: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE030");

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
