use recite_core::{Diagnostic, DiagnosticCode, DiagnosticSeverity, SourceSpan};

pub(crate) const EXPECTED_STATEMENT_OR_PROSE: &str = "RECITE_PARSE001";
pub(crate) const STATEMENT_BEFORE_BLOCK: &str = "RECITE_PARSE002";
pub(crate) const MISSING_BLOCK_ID: &str = "RECITE_PARSE003";
pub(crate) const EMPTY_BLOCK_ID: &str = "RECITE_PARSE005";
pub(crate) const MIXED_INDENT: &str = "RECITE_PARSE007";
pub(crate) const MALFORMED_HEADER: &str = "RECITE_PARSE008";
pub(crate) const MISSING_DIVERT_TARGET: &str = "RECITE_PARSE010";
pub(crate) const MALFORMED_DIVERT_TARGET: &str = "RECITE_PARSE011";
pub(crate) const MALFORMED_EFFECT: &str = "RECITE_PARSE012";
pub(crate) const MALFORMED_CONDITION: &str = "RECITE_PARSE013";
pub(crate) const MALFORMED_CASE: &str = "RECITE_PARSE014";
pub(crate) const MISPLACED_ELSE: &str = "RECITE_PARSE015";
pub(crate) const MISPLACED_CASE: &str = "RECITE_PARSE016";
pub(crate) const PROSE_AFTER_NESTED_STATEMENT: &str = "RECITE_PARSE017";

pub(crate) fn expected_statement_or_prose(span: SourceSpan) -> Diagnostic {
    diagnostic(
        EXPECTED_STATEMENT_OR_PROSE,
        "expected a Recite statement header or indented prose",
        span,
    )
}

pub(crate) fn statement_before_block(span: SourceSpan) -> Diagnostic {
    diagnostic(
        STATEMENT_BEFORE_BLOCK,
        "statement appears before a block header",
        span,
    )
}

pub(crate) fn missing_block_id(span: SourceSpan) -> Diagnostic {
    diagnostic(
        MISSING_BLOCK_ID,
        "block header must include a block id",
        span,
    )
}

pub(crate) fn empty_block_id(span: SourceSpan) -> Diagnostic {
    diagnostic(EMPTY_BLOCK_ID, "block id must not be empty", span)
}

pub(crate) fn mixed_indent(span: SourceSpan) -> Diagnostic {
    diagnostic(
        MIXED_INDENT,
        "mixed indentation inside statement body",
        span,
    )
}

pub(crate) fn malformed_header(span: SourceSpan) -> Diagnostic {
    diagnostic(MALFORMED_HEADER, "malformed statement header field", span)
}

pub(crate) fn missing_divert_target(span: SourceSpan) -> Diagnostic {
    diagnostic(
        MISSING_DIVERT_TARGET,
        "divert header must include a target",
        span,
    )
}

pub(crate) fn malformed_divert_target(span: SourceSpan) -> Diagnostic {
    diagnostic(MALFORMED_DIVERT_TARGET, "malformed divert target", span)
}

pub(crate) fn malformed_effect(span: SourceSpan, detail: impl AsRef<str>) -> Diagnostic {
    diagnostic(
        MALFORMED_EFFECT,
        format!("malformed effect statement: {}", detail.as_ref()),
        span,
    )
}

pub(crate) fn malformed_condition(span: SourceSpan, detail: impl AsRef<str>) -> Diagnostic {
    diagnostic(
        MALFORMED_CONDITION,
        format!("malformed condition expression: {}", detail.as_ref()),
        span,
    )
}

pub(crate) fn malformed_case(span: SourceSpan) -> Diagnostic {
    diagnostic(
        MALFORMED_CASE,
        "case header must include a variant or _",
        span,
    )
}

pub(crate) fn misplaced_else(span: SourceSpan) -> Diagnostic {
    diagnostic(
        MISPLACED_ELSE,
        ":else must immediately follow a sibling :if body",
        span,
    )
}

pub(crate) fn misplaced_case(span: SourceSpan) -> Diagnostic {
    diagnostic(
        MISPLACED_CASE,
        ":case must appear inside a :match body",
        span,
    )
}

pub(crate) fn prose_after_nested_statement(span: SourceSpan) -> Diagnostic {
    diagnostic(
        PROSE_AFTER_NESTED_STATEMENT,
        "prose cannot follow nested statements in the same body",
        span,
    )
}

fn diagnostic(code: &str, message: impl Into<String>, span: SourceSpan) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::new(code).expect("parser diagnostic codes are static and namespaced"),
        DiagnosticSeverity::Error,
        message,
        span,
    )
}
