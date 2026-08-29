use recite_core::{Choice, Diagnostic, DiagnosticCode, Line, LineId, SourceSpan, Statement};

use super::super::{
    auxiliary_presentation, compiler_diagnostic, diagnostic_contract, related_presentation,
    string_argument,
};
use super::span::SourceSpanOwner;

const INVALID_SOURCE_SPAN: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE008");
const MISSING_CHOICE_TARGET: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE012");
const UNSUPPORTED_LINE_CHILD_STATEMENT: DiagnosticCode =
    DiagnosticCode::new_static("RECITE_VALIDATE013");
const UNSUPPORTED_CHOICE_CHILD_STATEMENT: DiagnosticCode =
    DiagnosticCode::new_static("RECITE_VALIDATE014");
const UNKNOWN_CHOICE_ECHO_LINE: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE015");
const NON_FINITE_FLOAT_VALUE: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE016");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum SourceSpanError {
    FileMismatch,
    EndPrecedesStart,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) enum NonFiniteFloatOwner {
    ConditionArgument,
    EffectArgument,
    MetadataValue(String),
}

pub(crate) fn invalid_source_span(
    span: SourceSpan,
    owner: SourceSpanOwner,
    error: SourceSpanError,
) -> Diagnostic {
    let presentation_id = match error {
        SourceSpanError::FileMismatch => "diagnostic-validate-008-file",
        SourceSpanError::EndPrecedesStart => "diagnostic-validate-008-order",
    };
    compiler_diagnostic(
        diagnostic_contract(&INVALID_SOURCE_SPAN, presentation_id),
        format!(
            "invalid source span for {}: {}",
            owner.compatibility_label(),
            display_source_span_error(error)
        ),
        span,
        vec![(
            "owner".to_owned(),
            string_argument(owner.presentation_token()),
        )],
    )
}

pub(crate) fn missing_choice_target(choice: &Choice) -> Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&MISSING_CHOICE_TARGET, "diagnostic-validate-012"),
        "choice must target a block or END before it can be compiled",
        choice.span.clone(),
        [],
    )
    .with_help_presentation(auxiliary_presentation("diagnostic-validate-012-help", []))
}

pub(crate) fn unsupported_line_child_statement(line: &Line, statement: &Statement) -> Diagnostic {
    let statement_kind = StatementKindToken::from_statement(statement);
    compiler_diagnostic(
        diagnostic_contract(&UNSUPPORTED_LINE_CHILD_STATEMENT, "diagnostic-validate-013"),
        format!(
            "line `{}` contains a nested {} statement that v0 compiled prompts cannot represent",
            display_optional_line_id(line),
            statement_kind.as_str(),
        ),
        statement_span(statement).clone(),
        vec![
            (
                "line_id".to_owned(),
                string_argument(display_optional_line_id(line)),
            ),
            (
                "statement_kind".to_owned(),
                string_argument(statement_kind.as_str()),
            ),
        ],
    )
    .with_related_presentations([related_presentation(
        line.span.clone(),
        "diagnostic-validate-013-related",
        [],
    )])
    .with_help_presentation(auxiliary_presentation("diagnostic-validate-013-help", []))
}

pub(crate) fn unsupported_choice_child_statement(
    choice: &Choice,
    statement: &Statement,
) -> Diagnostic {
    let statement_kind = StatementKindToken::from_statement(statement);
    compiler_diagnostic(
        diagnostic_contract(
            &UNSUPPORTED_CHOICE_CHILD_STATEMENT,
            "diagnostic-validate-014",
        ),
        format!(
            "choice `{}` contains a nested {} statement that v0 compiled choices cannot represent",
            display_optional_choice_id(choice),
            statement_kind.as_str(),
        ),
        statement_span(statement).clone(),
        vec![
            (
                "choice_id".to_owned(),
                string_argument(display_optional_choice_id(choice)),
            ),
            (
                "statement_kind".to_owned(),
                string_argument(statement_kind.as_str()),
            ),
        ],
    )
    .with_related_presentations([related_presentation(
        choice.span.clone(),
        "diagnostic-validate-014-related",
        [],
    )])
    .with_help_presentation(auxiliary_presentation("diagnostic-validate-014-help", []))
}

pub(crate) fn unknown_choice_echo_line(choice: &Choice, line_id: &LineId) -> Diagnostic {
    compiler_diagnostic(
        diagnostic_contract(&UNKNOWN_CHOICE_ECHO_LINE, "diagnostic-validate-015"),
        format!("choice echo references unknown line id `{line_id}`"),
        choice.span.clone(),
        vec![("line_id".to_owned(), string_argument(line_id.to_string()))],
    )
    .with_help_presentation(auxiliary_presentation("diagnostic-validate-015-help", []))
}

pub(crate) fn non_finite_float_value(span: SourceSpan, owner: NonFiniteFloatOwner) -> Diagnostic {
    match owner {
        NonFiniteFloatOwner::ConditionArgument => compiler_diagnostic(
            diagnostic_contract(&NON_FINITE_FLOAT_VALUE, "diagnostic-validate-016-condition"),
            "condition argument contains a non-finite float value",
            span,
            [],
        )
        .with_help_presentation(auxiliary_presentation("diagnostic-validate-016-help", [])),
        NonFiniteFloatOwner::EffectArgument => compiler_diagnostic(
            diagnostic_contract(&NON_FINITE_FLOAT_VALUE, "diagnostic-validate-016-effect"),
            "effect argument contains a non-finite float value",
            span,
            [],
        )
        .with_help_presentation(auxiliary_presentation("diagnostic-validate-016-help", [])),
        NonFiniteFloatOwner::MetadataValue(key) => compiler_diagnostic(
            diagnostic_contract(&NON_FINITE_FLOAT_VALUE, "diagnostic-validate-016-metadata"),
            format!("metadata value `{key}` contains a non-finite float value"),
            span,
            vec![("key".to_owned(), string_argument(key))],
        )
        .with_help_presentation(auxiliary_presentation("diagnostic-validate-016-help", [])),
    }
}

fn display_source_span_error(error: SourceSpanError) -> &'static str {
    match error {
        SourceSpanError::FileMismatch => "span file does not match source file",
        SourceSpanError::EndPrecedesStart => "span end precedes span start",
    }
}

fn display_optional_line_id(line: &Line) -> String {
    line.id
        .as_ref()
        .map_or_else(|| "<missing>".to_owned(), ToString::to_string)
}

fn display_optional_choice_id(choice: &Choice) -> String {
    choice
        .id
        .as_ref()
        .map_or_else(|| "<missing>".to_owned(), ToString::to_string)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum StatementKindToken {
    Line,
    Choice,
    Divert,
    If,
    Match,
    Effect,
    Comment,
}

impl StatementKindToken {
    fn from_statement(statement: &Statement) -> Self {
        match statement {
            Statement::Line(_) => Self::Line,
            Statement::Choice(_) => Self::Choice,
            Statement::Divert(_) => Self::Divert,
            Statement::If(_) => Self::If,
            Statement::Match(_) => Self::Match,
            Statement::Effect(_) => Self::Effect,
            Statement::Comment(_) => Self::Comment,
        }
    }

    const fn as_str(self) -> &'static str {
        match self {
            Self::Line => "line",
            Self::Choice => "choice",
            Self::Divert => "divert",
            Self::If => "if",
            Self::Match => "match",
            Self::Effect => "effect",
            Self::Comment => "comment",
        }
    }
}

fn statement_span(statement: &Statement) -> &SourceSpan {
    match statement {
        Statement::Line(line) => &line.span,
        Statement::Choice(choice) => &choice.span,
        Statement::Divert(divert) => &divert.span,
        Statement::If(branch) => &branch.span,
        Statement::Match(branch) => &branch.span,
        Statement::Effect(effect) => &effect.span,
        Statement::Comment(comment) => &comment.span,
    }
}
