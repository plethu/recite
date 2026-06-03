use recite_core::{Diagnostic, DiagnosticCode, SourceSpan};

pub(crate) const EXPECTED_STATEMENT_OR_PROSE: DiagnosticCode =
    DiagnosticCode::new_static("RECITE_PARSE001");
pub(crate) const STATEMENT_BEFORE_BLOCK: DiagnosticCode =
    DiagnosticCode::new_static("RECITE_PARSE002");
pub(crate) const MISSING_BLOCK_ID: DiagnosticCode = DiagnosticCode::new_static("RECITE_PARSE003");
pub(crate) const EMPTY_BLOCK_ID: DiagnosticCode = DiagnosticCode::new_static("RECITE_PARSE005");
pub(crate) const MIXED_INDENT: DiagnosticCode = DiagnosticCode::new_static("RECITE_PARSE007");
pub(crate) const MALFORMED_HEADER: DiagnosticCode = DiagnosticCode::new_static("RECITE_PARSE008");
pub(crate) const MISSING_DIVERT_TARGET: DiagnosticCode =
    DiagnosticCode::new_static("RECITE_PARSE010");
pub(crate) const MALFORMED_DIVERT_TARGET: DiagnosticCode =
    DiagnosticCode::new_static("RECITE_PARSE011");
pub(crate) const MALFORMED_EFFECT: DiagnosticCode = DiagnosticCode::new_static("RECITE_PARSE012");
pub(crate) const MALFORMED_CONDITION: DiagnosticCode =
    DiagnosticCode::new_static("RECITE_PARSE013");
pub(crate) const MALFORMED_CASE: DiagnosticCode = DiagnosticCode::new_static("RECITE_PARSE014");
pub(crate) const MISPLACED_ELSE: DiagnosticCode = DiagnosticCode::new_static("RECITE_PARSE015");
pub(crate) const MISPLACED_CASE: DiagnosticCode = DiagnosticCode::new_static("RECITE_PARSE016");
pub(crate) const PROSE_AFTER_NESTED_STATEMENT: DiagnosticCode =
    DiagnosticCode::new_static("RECITE_PARSE017");
pub(crate) const TRAILING_CHOICE_IF: DiagnosticCode = DiagnosticCode::new_static("RECITE_PARSE018");

pub(crate) fn expected_statement_or_prose(span: SourceSpan) -> Diagnostic {
    Diagnostic::error(
        EXPECTED_STATEMENT_OR_PROSE,
        "expected a Recite statement header or indented prose",
        span,
    )
}

pub(crate) fn statement_before_block(span: SourceSpan) -> Diagnostic {
    Diagnostic::error(
        STATEMENT_BEFORE_BLOCK,
        "statement appears before a block header",
        span,
    )
}

pub(crate) fn missing_block_id(span: SourceSpan) -> Diagnostic {
    Diagnostic::error(
        MISSING_BLOCK_ID,
        "block header must include a block id",
        span,
    )
}

pub(crate) fn empty_block_id(span: SourceSpan) -> Diagnostic {
    Diagnostic::error(EMPTY_BLOCK_ID, "block id must not be empty", span)
}

pub(crate) fn mixed_indent(span: SourceSpan) -> Diagnostic {
    Diagnostic::error(
        MIXED_INDENT,
        "mixed indentation inside statement body",
        span,
    )
}

pub(crate) fn malformed_header(span: SourceSpan) -> Diagnostic {
    Diagnostic::error(MALFORMED_HEADER, "malformed statement header field", span)
}

pub(crate) fn missing_divert_target(span: SourceSpan) -> Diagnostic {
    Diagnostic::error(
        MISSING_DIVERT_TARGET,
        "divert header must include a target",
        span,
    )
}

pub(crate) fn malformed_divert_target(span: SourceSpan) -> Diagnostic {
    Diagnostic::error(MALFORMED_DIVERT_TARGET, "malformed divert target", span)
}

pub(crate) fn malformed_effect(span: SourceSpan, detail: impl AsRef<str>) -> Diagnostic {
    Diagnostic::error(
        MALFORMED_EFFECT,
        format!("malformed effect statement: {}", detail.as_ref()),
        span,
    )
}

pub(crate) fn malformed_condition(span: SourceSpan, detail: impl AsRef<str>) -> Diagnostic {
    Diagnostic::error(
        MALFORMED_CONDITION,
        format!("malformed condition expression: {}", detail.as_ref()),
        span,
    )
}

pub(crate) fn trailing_choice_if(span: SourceSpan) -> Diagnostic {
    Diagnostic::error(
        TRAILING_CHOICE_IF,
        "old trailing choice if syntax is not valid Recite v1 syntax",
        span,
    )
    .with_help("use requires=(...) for visible unavailable choices or :if for hidden choices")
}

pub(crate) fn malformed_case(span: SourceSpan) -> Diagnostic {
    Diagnostic::error(
        MALFORMED_CASE,
        "case header must include a variant or _",
        span,
    )
}

pub(crate) fn misplaced_else(span: SourceSpan) -> Diagnostic {
    Diagnostic::error(
        MISPLACED_ELSE,
        ":else must immediately follow a sibling :if body",
        span,
    )
}

pub(crate) fn misplaced_case(span: SourceSpan) -> Diagnostic {
    Diagnostic::error(
        MISPLACED_CASE,
        ":case must appear inside a :match body",
        span,
    )
}

pub(crate) fn prose_after_nested_statement(span: SourceSpan) -> Diagnostic {
    Diagnostic::error(
        PROSE_AFTER_NESTED_STATEMENT,
        "prose cannot follow nested statements in the same body",
        span,
    )
}
