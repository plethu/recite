use recite_core::{Diagnostic, DiagnosticCode, EffectMode, SchemaTypeRef, SourceSpan};

use super::display_schema_type_ref;

const UNKNOWN_EFFECT_FUNCTION: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE017");
const WRONG_EFFECT_ARITY: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE018");
const WRONG_EFFECT_ARGUMENT_TYPE: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE019");
const UNSUPPORTED_EFFECT_MODE: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE020");
const INVALID_EFFECT_ARGUMENT_VALUE: DiagnosticCode =
    DiagnosticCode::new_static("RECITE_VALIDATE021");

pub(crate) fn unknown_effect_function(function: &str, span: SourceSpan) -> Diagnostic {
    Diagnostic::error(
        UNKNOWN_EFFECT_FUNCTION,
        format!("unknown effect function `{function}`"),
        span,
    )
    .with_help("declare the effect in the project schema manifest")
}

pub(crate) fn wrong_effect_arity(
    function: &str,
    expected: usize,
    actual: usize,
    span: SourceSpan,
) -> Diagnostic {
    Diagnostic::error(
        WRONG_EFFECT_ARITY,
        format!(
            "effect `{function}` expects {expected} argument{}, but got {actual}",
            if expected == 1 { "" } else { "s" }
        ),
        span,
    )
    .with_help("match the effect parameters declared in the project schema manifest")
}

pub(crate) fn wrong_effect_argument_type(
    function: &str,
    index: usize,
    expected: &SchemaTypeRef,
    actual: &str,
    span: SourceSpan,
) -> Diagnostic {
    Diagnostic::error(
        WRONG_EFFECT_ARGUMENT_TYPE,
        format!(
            "argument {} for effect `{function}` expects {}, but got {actual}",
            index + 1,
            display_schema_type_ref(expected),
        ),
        span,
    )
}

pub(crate) fn unsupported_effect_mode(
    function: &str,
    mode: EffectMode,
    span: SourceSpan,
) -> Diagnostic {
    Diagnostic::error(
        UNSUPPORTED_EFFECT_MODE,
        format!(
            "effect `{function}` does not support {} mode",
            display_effect_mode(mode)
        ),
        span,
    )
    .with_help("use a mode declared for this effect in the project schema manifest")
}

pub(crate) fn invalid_effect_argument_value(
    function: &str,
    index: usize,
    expected: &SchemaTypeRef,
    value: &str,
    span: SourceSpan,
) -> Diagnostic {
    Diagnostic::error(
        INVALID_EFFECT_ARGUMENT_VALUE,
        format!(
            "argument {} for effect `{function}` uses unknown {} value `{value}`",
            index + 1,
            display_schema_type_ref(expected),
        ),
        span,
    )
    .with_help("use a value exported in the project schema manifest")
}

fn display_effect_mode(mode: EffectMode) -> &'static str {
    match mode {
        EffectMode::Deferred => "deferred",
        EffectMode::Immediate => "immediate",
        EffectMode::Blocking => "blocking",
    }
}
