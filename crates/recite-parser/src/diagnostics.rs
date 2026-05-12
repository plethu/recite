use recite_core::{Diagnostic, DiagnosticCode, DiagnosticSeverity, SourceSpan};

pub(crate) const EXPECTED_STATEMENT_OR_PROSE: &str = "RECITE_PARSE001";
pub(crate) const STATEMENT_BEFORE_BLOCK: &str = "RECITE_PARSE002";
pub(crate) const MISSING_BLOCK_ID: &str = "RECITE_PARSE003";
pub(crate) const UNSUPPORTED_LOWERING: &str = "RECITE_PARSE004";
pub(crate) const EMPTY_BLOCK_ID: &str = "RECITE_PARSE005";
pub(crate) const MISSING_LINE_ID: &str = "RECITE_PARSE006";

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

pub(crate) fn unsupported_lowering(span: SourceSpan) -> Diagnostic {
    diagnostic(
        UNSUPPORTED_LOWERING,
        "statement syntax is parsed losslessly but lowering is not implemented yet",
        span,
    )
}

pub(crate) fn nested_unsupported_lowering(span: SourceSpan) -> Diagnostic {
    diagnostic(
        UNSUPPORTED_LOWERING,
        "nested statement syntax is parsed losslessly but lowering is not implemented yet",
        span,
    )
}

pub(crate) fn empty_block_id(span: SourceSpan) -> Diagnostic {
    diagnostic(EMPTY_BLOCK_ID, "block id must not be empty", span)
}

pub(crate) fn missing_line_id(span: SourceSpan) -> Diagnostic {
    diagnostic(
        MISSING_LINE_ID,
        "line header has no line id; semantic validation may require one",
        span,
    )
}

fn diagnostic(code: &str, message: &str, span: SourceSpan) -> Diagnostic {
    Diagnostic::new(
        DiagnosticCode::new(code).expect("parser diagnostic codes are static and namespaced"),
        DiagnosticSeverity::Error,
        message,
        span,
    )
}
