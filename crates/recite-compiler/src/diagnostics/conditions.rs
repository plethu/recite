use recite_core::{ConditionReturnType, Diagnostic, DiagnosticCode, SchemaTypeRef, SourceSpan};

use super::display_schema_type_ref;

const UNKNOWN_CONDITION_FUNCTION: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE034");
const WRONG_CONDITION_ARITY: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE035");
const WRONG_CONDITION_ARGUMENT_TYPE: DiagnosticCode =
    DiagnosticCode::new_static("RECITE_VALIDATE036");
const INVALID_CONDITION_ARGUMENT_VALUE: DiagnosticCode =
    DiagnosticCode::new_static("RECITE_VALIDATE037");
const WRONG_CONDITION_RETURN_TYPE: DiagnosticCode =
    DiagnosticCode::new_static("RECITE_VALIDATE038");
const UNKNOWN_AVAILABILITY_REASON_OVERRIDE: DiagnosticCode =
    DiagnosticCode::new_static("RECITE_VALIDATE039");
const PARAMETERIZED_AVAILABILITY_REASON_OVERRIDE: DiagnosticCode =
    DiagnosticCode::new_static("RECITE_VALIDATE040");
const AVAILABILITY_REASON_WITHOUT_REQUIREMENT: DiagnosticCode =
    DiagnosticCode::new_static("RECITE_VALIDATE041");

pub(crate) fn unknown_condition_function(function: &str, span: SourceSpan) -> Diagnostic {
    Diagnostic::error(
        UNKNOWN_CONDITION_FUNCTION,
        format!("unknown condition function `{function}`"),
        span,
    )
    .with_help("declare the condition in the project schema manifest")
}

pub(crate) fn wrong_condition_arity(
    function: &str,
    expected: usize,
    actual: usize,
    span: SourceSpan,
) -> Diagnostic {
    Diagnostic::error(
        WRONG_CONDITION_ARITY,
        format!(
            "condition `{function}` expects {expected} argument{}, but got {actual}",
            if expected == 1 { "" } else { "s" }
        ),
        span,
    )
    .with_help("match the condition parameters declared in the project schema manifest")
}

pub(crate) fn wrong_condition_argument_type(
    function: &str,
    index: usize,
    expected: &SchemaTypeRef,
    actual: &str,
    span: SourceSpan,
) -> Diagnostic {
    Diagnostic::error(
        WRONG_CONDITION_ARGUMENT_TYPE,
        format!(
            "argument {} for condition `{function}` expects {}, but got {actual}",
            index + 1,
            display_schema_type_ref(expected),
        ),
        span,
    )
}

pub(crate) fn invalid_condition_argument_value(
    function: &str,
    index: usize,
    expected: &SchemaTypeRef,
    value: &str,
    span: SourceSpan,
) -> Diagnostic {
    Diagnostic::error(
        INVALID_CONDITION_ARGUMENT_VALUE,
        format!(
            "argument {} for condition `{function}` uses unknown {} value `{value}`",
            index + 1,
            display_schema_type_ref(expected),
        ),
        span,
    )
    .with_help("use a value exported in the project schema manifest")
}

pub(crate) fn wrong_condition_return_type(
    function: &str,
    expected: &str,
    actual: &ConditionReturnType,
    span: SourceSpan,
) -> Diagnostic {
    Diagnostic::error(
        WRONG_CONDITION_RETURN_TYPE,
        format!(
            "condition `{function}` returns {}, but {expected} is required",
            display_condition_return_type(actual),
        ),
        span,
    )
}

pub(crate) fn unknown_availability_reason_override(reason: &str, span: SourceSpan) -> Diagnostic {
    Diagnostic::error(
        UNKNOWN_AVAILABILITY_REASON_OVERRIDE,
        format!("unknown availability reason `{reason}`"),
        span,
    )
    .with_help("declare the availability reason in the project schema manifest")
}

pub(crate) fn parameterized_availability_reason_override(
    reason: &str,
    span: SourceSpan,
) -> Diagnostic {
    Diagnostic::error(
        PARAMETERIZED_AVAILABILITY_REASON_OVERRIDE,
        format!("availability reason override `{reason}` must be parameterless"),
        span,
    )
    .with_help("v1 reason= overrides cannot bind parameters")
}

pub(crate) fn availability_reason_without_requirement(
    reason: &str,
    span: SourceSpan,
) -> Diagnostic {
    Diagnostic::error(
        AVAILABILITY_REASON_WITHOUT_REQUIREMENT,
        format!("availability reason `{reason}` requires a choice requires=(...) clause"),
        span,
    )
}

fn display_condition_return_type(return_type: &ConditionReturnType) -> String {
    match return_type {
        ConditionReturnType::Bool => "bool".to_owned(),
        ConditionReturnType::Enum(name) => format!("enum:{name}"),
    }
}
