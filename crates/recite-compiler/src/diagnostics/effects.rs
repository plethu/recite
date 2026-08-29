use recite_core::{Diagnostic, DiagnosticCode, EffectMode, SchemaTypeRef, SourceSpan};

use super::{compiler_diagnostic, diagnostic_contract, integer_argument, string_argument};

const UNKNOWN_EFFECT_FUNCTION: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE017");
const WRONG_EFFECT_ARITY: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE018");
const WRONG_EFFECT_ARGUMENT_TYPE: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE019");
const UNSUPPORTED_EFFECT_MODE: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE020");
const INVALID_EFFECT_ARGUMENT_VALUE: DiagnosticCode =
    DiagnosticCode::new_static("RECITE_VALIDATE021");

pub(crate) fn unknown_effect_function(function: &str, span: SourceSpan) -> Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&UNKNOWN_EFFECT_FUNCTION, "diagnostic-validate-017"),
        format!("unknown effect function `{function}`"),
        span,
        vec![("function".to_owned(), string_argument(function))],
    )
    .with_help_presentation(super::auxiliary_presentation(
        "diagnostic-validate-017-help",
        [],
    ))
}

pub(crate) fn wrong_effect_arity(
    function: &str,
    expected: usize,
    actual: usize,
    span: SourceSpan,
) -> Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&WRONG_EFFECT_ARITY, "diagnostic-validate-018"),
        format!(
            "effect `{function}` expects {expected} argument{}, but got {actual}",
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
        "diagnostic-validate-018-help",
        [],
    ))
}

pub(crate) fn wrong_effect_argument_type(
    function: &str,
    index: usize,
    expected: &SchemaTypeRef,
    actual: &str,
    span: SourceSpan,
) -> Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&WRONG_EFFECT_ARGUMENT_TYPE, "diagnostic-validate-019"),
        format!(
            "argument {} for effect `{function}` expects {}, but got {actual}",
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

pub(crate) fn unsupported_effect_mode(
    function: &str,
    mode: EffectMode,
    span: SourceSpan,
) -> Diagnostic {
    let mode = display_effect_mode(mode);
    compiler_diagnostic(
        diagnostic_contract(&UNSUPPORTED_EFFECT_MODE, "diagnostic-validate-020"),
        format!("effect `{function}` does not support {mode} mode"),
        span,
        vec![
            ("function".to_owned(), string_argument(function)),
            ("mode".to_owned(), string_argument(mode)),
        ],
    )
    .with_help_presentation(super::auxiliary_presentation(
        "diagnostic-validate-020-help",
        [],
    ))
}

pub(crate) fn invalid_effect_argument_value(
    function: &str,
    index: usize,
    expected: &SchemaTypeRef,
    value: &str,
    span: SourceSpan,
) -> Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&INVALID_EFFECT_ARGUMENT_VALUE, "diagnostic-validate-021"),
        format!(
            "argument {} for effect `{function}` uses unknown {} value `{value}`",
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
        "diagnostic-validate-021-help",
        [],
    ))
}

fn display_schema_type_ref(type_ref: &SchemaTypeRef) -> String {
    super::display_schema_type_ref(type_ref)
}

fn display_effect_mode(mode: EffectMode) -> &'static str {
    match mode {
        EffectMode::Deferred => "deferred",
        EffectMode::Immediate => "immediate",
        EffectMode::Blocking => "blocking",
    }
}
