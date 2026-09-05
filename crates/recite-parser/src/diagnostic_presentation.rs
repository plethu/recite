use recite_core::{
    Diagnostic, DiagnosticArgumentValue, DiagnosticCode, DiagnosticPresentation,
    DiagnosticPresentationId, SourceSpan, contract_for, default_presentation_id_for_code,
    explain_diagnostic_code,
};

use crate::condition::ParseErrorKind;

pub(super) fn static_diagnostic(
    code: DiagnosticCode,
    message: &str,
    span: SourceSpan,
) -> Diagnostic {
    let presentation_id = default_presentation_id_for_code(&code);
    diagnostic_with_presentation(code, presentation_id, message.to_owned(), span, Vec::new())
}

pub(super) fn dynamic_diagnostic(
    code: DiagnosticCode,
    message: String,
    span: SourceSpan,
    selector: DiagnosticSelector,
) -> Diagnostic {
    let presentation_id = selector.presentation_id(&code);
    let arguments = selector.arguments();
    diagnostic_with_presentation(code, presentation_id, message, span, arguments)
}

#[derive(Clone, Copy)]
pub(super) enum DiagnosticSelector {
    MissingEffectMode,
    InvalidEffectMode,
    ParseError(ParseErrorKind),
}

impl DiagnosticSelector {
    fn presentation_id(self, code: &DiagnosticCode) -> DiagnosticPresentationId {
        match self {
            Self::ParseError(ParseErrorKind::UnexpectedCharacter(_)) => match code.as_str() {
                "RECITE_PARSE012" => DiagnosticPresentationId::new_static(
                    "diagnostic-parse-012-unexpected-character",
                ),
                "RECITE_PARSE013" => DiagnosticPresentationId::new_static(
                    "diagnostic-parse-013-unexpected-character",
                ),
                _ => default_presentation_id_for_code(code),
            },
            _ => default_presentation_id_for_code(code),
        }
    }

    fn arguments(self) -> Vec<(String, DiagnosticArgumentValue)> {
        match self {
            Self::MissingEffectMode => reason_argument("missing_mode"),
            Self::InvalidEffectMode => reason_argument("invalid_mode"),
            Self::ParseError(ParseErrorKind::UnexpectedCharacter(character)) => vec![(
                "character".to_owned(),
                DiagnosticArgumentValue::String(escaped_character(character)),
            )],
            Self::ParseError(error_kind) => reason_argument(parse_selector(error_kind)),
        }
    }
}

fn reason_argument(reason: &str) -> Vec<(String, DiagnosticArgumentValue)> {
    vec![(
        "reason".to_owned(),
        DiagnosticArgumentValue::String(reason.to_owned()),
    )]
}

#[allow(
    clippy::expect_used,
    reason = "this helper owns parser diagnostic contract lookup and argument validation"
)]
fn diagnostic_with_presentation(
    code: DiagnosticCode,
    presentation_id: DiagnosticPresentationId,
    message: String,
    span: SourceSpan,
    arguments: Vec<(String, DiagnosticArgumentValue)>,
) -> Diagnostic {
    let contract = contract_for(&code, &presentation_id)
        .expect("every parser diagnostic must have a producer presentation contract");
    let mut diagnostic = Diagnostic::error_from_contract(contract, message, span, arguments)
        .expect("parser diagnostic arguments must match their central contract");
    if let Some(explanation) = explain_diagnostic_code(&code) {
        diagnostic = diagnostic.with_explanation_presentation(explanation.presentation());
    }
    diagnostic
}

pub(super) fn explanation_remediation(code: DiagnosticCode) -> Option<DiagnosticPresentation> {
    explain_diagnostic_code(&code)
        .and_then(|explanation| explanation.presentation().remediation.into_iter().next())
}

fn parse_selector(error_kind: ParseErrorKind) -> &'static str {
    match error_kind {
        ParseErrorKind::UnexpectedCharacter(_) => {
            unreachable!("unexpected characters use their exact character presentation contract")
        }
        ParseErrorKind::UnterminatedString => "unterminated_string",
        ParseErrorKind::InvalidFloat => "invalid_float",
        ParseErrorKind::InvalidInteger => "invalid_integer",
        ParseErrorKind::ExpectedFunctionCall => "expected_function_call",
        ParseErrorKind::ExpectedFunctionNameParen => "expected_function_name_paren",
        ParseErrorKind::ExpectedRightParen => "expected_right_paren",
        ParseErrorKind::ExpectedScalarArgument => "expected_scalar_argument",
        ParseErrorKind::UnexpectedTrailingTokens => "unexpected_trailing_tokens",
    }
}

fn escaped_character(character: char) -> String {
    character.escape_debug().to_string()
}
