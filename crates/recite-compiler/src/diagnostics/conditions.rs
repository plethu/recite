use recite_core::{ConditionReturnType, Diagnostic, DiagnosticCode, SchemaTypeRef, SourceSpan};

use super::{compiler_diagnostic, diagnostic_contract, integer_argument, string_argument};

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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum ConditionReturnRequirement {
    Bool,
    Enum,
}

impl ConditionReturnRequirement {
    const fn presentation_id(self) -> &'static str {
        match self {
            Self::Bool => "diagnostic-validate-038-bool",
            Self::Enum => "diagnostic-validate-038-enum",
        }
    }

    const fn compatibility_label(self) -> &'static str {
        match self {
            Self::Bool => "bool",
            Self::Enum => "enum",
        }
    }
}

pub(crate) fn unknown_condition_function(function: &str, span: SourceSpan) -> Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&UNKNOWN_CONDITION_FUNCTION, "diagnostic-validate-034"),
        format!("unknown condition function `{function}`"),
        span,
        vec![("function".to_owned(), string_argument(function))],
    )
    .with_help_presentation(super::auxiliary_presentation(
        "diagnostic-validate-034-help",
        [],
    ))
}

pub(crate) fn wrong_condition_arity(
    function: &str,
    expected: usize,
    actual: usize,
    span: SourceSpan,
) -> Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&WRONG_CONDITION_ARITY, "diagnostic-validate-035"),
        format!(
            "condition `{function}` expects {expected} argument{}, but got {actual}",
            if expected == 1 { "" } else { "s" }
        ),
        span,
        vec![
            ("function".to_owned(), string_argument(function)),
            ("expected".to_owned(), integer_argument(expected)),
            ("actual".to_owned(), integer_argument(actual)),
        ],
    )
    .with_help_presentation(super::auxiliary_presentation(
        "diagnostic-validate-035-help",
        [],
    ))
}

pub(crate) fn wrong_condition_argument_type(
    function: &str,
    index: usize,
    expected: &SchemaTypeRef,
    actual: &str,
    span: SourceSpan,
) -> Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&WRONG_CONDITION_ARGUMENT_TYPE, "diagnostic-validate-036"),
        format!(
            "argument {} for condition `{function}` expects {}, but got {actual}",
            index + 1,
            display_schema_type_ref(expected),
        ),
        span,
        vec![
            ("function".to_owned(), string_argument(function)),
            ("index".to_owned(), integer_argument(index + 1)),
            (
                "expected".to_owned(),
                string_argument(display_schema_type_ref(expected)),
            ),
            ("actual".to_owned(), string_argument(actual)),
        ],
    )
}

pub(crate) fn invalid_condition_argument_value(
    function: &str,
    index: usize,
    expected: &SchemaTypeRef,
    value: &str,
    span: SourceSpan,
) -> Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&INVALID_CONDITION_ARGUMENT_VALUE, "diagnostic-validate-037"),
        format!(
            "argument {} for condition `{function}` uses unknown {} value `{value}`",
            index + 1,
            display_schema_type_ref(expected),
        ),
        span,
        vec![
            ("function".to_owned(), string_argument(function)),
            ("index".to_owned(), integer_argument(index + 1)),
            (
                "expected".to_owned(),
                string_argument(display_schema_type_ref(expected)),
            ),
            ("value".to_owned(), string_argument(value)),
        ],
    )
    .with_help_presentation(super::auxiliary_presentation(
        "diagnostic-validate-037-help",
        [],
    ))
}

pub(crate) fn wrong_condition_return_type(
    function: &str,
    expected: ConditionReturnRequirement,
    actual: &ConditionReturnType,
    span: SourceSpan,
) -> Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&WRONG_CONDITION_RETURN_TYPE, expected.presentation_id()),
        format!(
            "condition `{function}` returns {}, but {} is required",
            display_condition_return_type(actual),
            expected.compatibility_label(),
        ),
        span,
        vec![
            ("function".to_owned(), string_argument(function)),
            (
                "actual".to_owned(),
                string_argument(display_condition_return_type(actual)),
            ),
        ],
    )
}

pub(crate) fn unknown_availability_reason_override(reason: &str, span: SourceSpan) -> Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(
            &UNKNOWN_AVAILABILITY_REASON_OVERRIDE,
            "diagnostic-validate-039",
        ),
        format!("unknown availability reason `{reason}`"),
        span,
        vec![("reason".to_owned(), string_argument(reason))],
    )
    .with_help_presentation(super::auxiliary_presentation(
        "diagnostic-validate-039-help",
        [],
    ))
}

pub(crate) fn parameterized_availability_reason_override(
    reason: &str,
    span: SourceSpan,
) -> Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(
            &PARAMETERIZED_AVAILABILITY_REASON_OVERRIDE,
            "diagnostic-validate-040",
        ),
        format!("availability reason override `{reason}` must be parameterless"),
        span,
        vec![("reason".to_owned(), string_argument(reason))],
    )
    .with_help_presentation(super::auxiliary_presentation(
        "diagnostic-validate-040-help",
        [],
    ))
}

pub(crate) fn availability_reason_without_requirement(
    reason: &str,
    span: SourceSpan,
) -> Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(
            &AVAILABILITY_REASON_WITHOUT_REQUIREMENT,
            "diagnostic-validate-041",
        ),
        format!("availability reason `{reason}` requires a choice requires=(...) clause"),
        span,
        vec![("reason".to_owned(), string_argument(reason))],
    )
}

fn display_condition_return_type(return_type: &ConditionReturnType) -> String {
    match return_type {
        ConditionReturnType::Bool => "bool".to_owned(),
        ConditionReturnType::Enum(name) => format!("enum:{name}"),
    }
}

fn display_schema_type_ref(type_ref: &SchemaTypeRef) -> String {
    super::display_schema_type_ref(type_ref)
}
