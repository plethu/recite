//! Syntax diagnostics and recovery shared by lossless parsing and direct lowering.
use crate::{
    body::{BodyBoundary, BodyCursor, BodyStep},
    diagnostics::expected_statement_or_prose,
    layout::{ClassifiedLine, classify_line},
    lower::mark_all,
    markers::StatementMarker,
    source::{LogicalLine, span_for_line},
};
use recite_core::{Diagnostic, SourceRecovery};

pub(super) fn scan(path: &str, lines: &[LogicalLine<'_>]) -> (Vec<Diagnostic>, SourceRecovery) {
    let mut diagnostics = Vec::new();
    let mut recovery = SourceRecovery::complete();
    for line in lines.iter().copied() {
        let classification = classify_line(line);
        if classification == ClassifiedLine::Error {
            mark_all(&mut recovery);
            if !line.trimmed_content().is_empty() {
                diagnostics.push(expected_statement_or_prose(span_for_line(
                    path,
                    line.number,
                    line.indent_len() + 1,
                )));
            }
        }
    }
    validate_body_indentation(path, lines, &mut diagnostics, &mut recovery);
    (diagnostics, recovery)
}

fn validate_body_indentation(
    path: &str,
    lines: &[LogicalLine<'_>],
    diagnostics: &mut Vec<Diagnostic>,
    recovery: &mut SourceRecovery,
) {
    for (index, line) in lines.iter().copied().enumerate() {
        if classify_line(line) != ClassifiedLine::Statement(StatementMarker::Line) {
            continue;
        }

        validate_line_body_indentation(path, lines, index, diagnostics, recovery);
    }
}

fn validate_line_body_indentation(
    path: &str,
    lines: &[LogicalLine<'_>],
    header_index: usize,
    diagnostics: &mut Vec<Diagnostic>,
    recovery: &mut SourceRecovery,
) {
    let mut cursor = BodyCursor::new(lines, header_index, BodyBoundary::HeaderIndent);

    while cursor.index() < lines.len() {
        let line = lines[cursor.index()];
        match cursor.step(path, line, true, diagnostics) {
            BodyStep::Content { .. } => {
                if matches!(classify_line(line), ClassifiedLine::Statement(_)) {
                    break;
                }
                cursor.advance();
            }
            BodyStep::Boundary => break,
            BodyStep::Blank => {}
            BodyStep::MixedIndent => mark_all(recovery),
        }
    }
}
