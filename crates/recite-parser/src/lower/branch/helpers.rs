use recite_core::{ConditionCall, ConditionExpression, Diagnostic, SourceSpan};

use crate::condition::{parse_condition_call, parse_condition_expression};
use crate::diagnostics::malformed_condition;
use crate::header::rest_after_prefix;
use crate::markers::StatementMarker;
use crate::source::{LogicalLine, span_for_line};

pub(super) struct DirectiveHeader<'source> {
    pub(super) line: u32,
    pub(super) column: usize,
    pub(super) text: &'source str,
    pub(super) span: SourceSpan,
}

pub(super) fn directive_header<'source>(
    path: &str,
    line: LogicalLine<'source>,
    marker: StatementMarker,
) -> DirectiveHeader<'source> {
    let indent = line.indent_len();
    let base_column = indent + 1;
    let rest = rest_after_prefix(line.trimmed_content(), marker.text(), base_column);

    DirectiveHeader {
        line: line.number,
        column: rest.column,
        text: rest.text,
        span: span_for_line(path, line.number, base_column),
    }
}

pub(super) fn parse_condition_expression_header(
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
    header: &DirectiveHeader<'_>,
) -> Option<ConditionExpression> {
    match parse_condition_expression(path, header.line, header.column, header.text) {
        Ok(condition) => Some(condition),
        Err(error) => {
            diagnostics.push(malformed_condition(error.span, error.message));
            None
        }
    }
}

pub(super) fn parse_condition_call_header(
    path: &str,
    diagnostics: &mut Vec<Diagnostic>,
    header: &DirectiveHeader<'_>,
) -> Option<ConditionCall> {
    match parse_condition_call(path, header.line, header.column, header.text) {
        Ok(call) => Some(call),
        Err(error) => {
            diagnostics.push(malformed_condition(error.span, error.message));
            None
        }
    }
}
