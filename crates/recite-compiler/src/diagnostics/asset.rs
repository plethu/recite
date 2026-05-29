use recite_core::{Choice, Diagnostic, DiagnosticCode, Line, LineId, SourceSpan, Statement};

const INVALID_SOURCE_SPAN: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE008");
const MISSING_CHOICE_TARGET: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE012");
const UNSUPPORTED_LINE_CHILD_STATEMENT: DiagnosticCode =
    DiagnosticCode::new_static("RECITE_VALIDATE013");
const UNSUPPORTED_CHOICE_CHILD_STATEMENT: DiagnosticCode =
    DiagnosticCode::new_static("RECITE_VALIDATE014");
const UNKNOWN_CHOICE_ECHO_LINE: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE015");
const NON_FINITE_FLOAT_VALUE: DiagnosticCode = DiagnosticCode::new_static("RECITE_VALIDATE016");

pub(crate) fn invalid_source_span(span: SourceSpan, owner: &str, detail: &str) -> Diagnostic {
    Diagnostic::error(
        INVALID_SOURCE_SPAN,
        format!("invalid source span for {owner}: {detail}"),
        span,
    )
}

pub(crate) fn missing_choice_target(choice: &Choice) -> Diagnostic {
    Diagnostic::error(
        MISSING_CHOICE_TARGET,
        "choice must target a block or END before it can be compiled",
        choice.span.clone(),
    )
    .with_help("add a choice body divert such as `-> next_block` or `-> END`")
}

pub(crate) fn unsupported_line_child_statement(line: &Line, statement: &Statement) -> Diagnostic {
    Diagnostic::error(
        UNSUPPORTED_LINE_CHILD_STATEMENT,
        format!(
            "line `{}` contains a nested {} statement that v0 compiled prompts cannot represent",
            display_optional_line_id(line),
            display_statement_kind(statement),
        ),
        statement_span(statement).clone(),
    )
    .with_related([recite_core::RelatedSpan::new(
        line.span.clone(),
        "line containing the unsupported nested statement is here",
    )])
    .with_help("keep only nested choices under prompt lines for v0 compiled assets")
}

pub(crate) fn unsupported_choice_child_statement(
    choice: &Choice,
    statement: &Statement,
) -> Diagnostic {
    Diagnostic::error(
        UNSUPPORTED_CHOICE_CHILD_STATEMENT,
        format!(
            "choice `{}` contains a nested {} statement that v0 compiled choices cannot represent",
            display_optional_choice_id(choice),
            display_statement_kind(statement),
        ),
        statement_span(statement).clone(),
    )
    .with_related([recite_core::RelatedSpan::new(
        choice.span.clone(),
        "choice containing the unsupported nested statement is here",
    )])
    .with_help("keep choice bodies to text and one target divert for v0 compiled assets")
}

pub(crate) fn unknown_choice_echo_line(choice: &Choice, line_id: &LineId) -> Diagnostic {
    Diagnostic::error(
        UNKNOWN_CHOICE_ECHO_LINE,
        format!("choice echo references unknown line id `{line_id}`"),
        choice.span.clone(),
    )
    .with_help("use an existing line ID, `echo=selected_text`, or `echo=none`")
}

pub(crate) fn non_finite_float_value(
    span: SourceSpan,
    owner: impl std::fmt::Display,
) -> Diagnostic {
    Diagnostic::error(
        NON_FINITE_FLOAT_VALUE,
        format!("{owner} contains a non-finite float value"),
        span,
    )
    .with_help("use a finite number so MessagePack and inspection JSON stay equivalent")
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

fn display_statement_kind(statement: &Statement) -> &'static str {
    match statement {
        Statement::Line(_) => "line",
        Statement::Choice(_) => "choice",
        Statement::Divert(_) => "divert",
        Statement::If(_) => "if",
        Statement::Match(_) => "match",
        Statement::Effect(_) => "effect",
        Statement::Comment(_) => "comment",
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
