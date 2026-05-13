use recite_core::Statement;

use crate::body::{BodyBoundary, BodyCursor, BodyStep};
use crate::diagnostics::{expected_statement_or_prose, prose_after_nested_statement};
use crate::layout::{ClassifiedLine, classify_line};
use crate::source::span_for_line;

use super::{LoweredProseBody, Lowerer};

impl Lowerer<'_, '_> {
    pub(super) fn lower_prose_body(
        &mut self,
        header_index: usize,
        emit_mixed_indent: bool,
    ) -> LoweredProseBody {
        let header = self.lines[header_index];
        let header_indent = header.indent_len();
        let mut text_start_line = header.number;
        let mut text_start_column = header_indent + 3;
        let mut text_lines = Vec::new();
        let mut statements = Vec::new();
        let mut saw_statement = false;
        let mut cursor = BodyCursor::new(&self.lines, header_index, BodyBoundary::HeaderIndent);

        while cursor.index() < self.lines.len() {
            let line = self.lines[cursor.index()];
            let index = match cursor.step(self.path, line, emit_mixed_indent, self.diagnostics) {
                BodyStep::Content { index } => index,
                BodyStep::Boundary => break,
                BodyStep::Blank => {
                    if !text_lines.is_empty() && !saw_statement {
                        text_lines.push(String::new());
                    }
                    continue;
                }
                BodyStep::MixedIndent => continue,
            };

            let trimmed = line.trimmed_content();
            let indent = line.indent_len();

            if matches!(classify_line(line), ClassifiedLine::Statement(_)) {
                trim_trailing_blank_lines(&mut text_lines);
                saw_statement = true;
                let (statement, next_index) = self.lower_statement(index);
                if let Some(statement) = statement {
                    statements.push(statement);
                }
                cursor.set_index(next_index);
                continue;
            }

            if saw_statement {
                self.diagnostics
                    .push(prose_after_nested_statement(span_for_line(
                        self.path,
                        line.number,
                        indent + 1,
                    )));
                cursor.advance();
                continue;
            }

            if text_lines.is_empty() {
                text_start_line = line.number;
                text_start_column = indent + 1;
            }
            text_lines.push(trimmed.to_owned());
            cursor.advance();
        }

        trim_trailing_blank_lines(&mut text_lines);

        LoweredProseBody {
            text: text_lines.join("\n"),
            text_span: span_for_line(self.path, text_start_line, text_start_column),
            statements,
            next_index: cursor.index(),
        }
    }

    pub(super) fn lower_statement_body(&mut self, header_index: usize) -> (Vec<Statement>, usize) {
        let mut statements = Vec::new();
        let mut cursor = BodyCursor::new(&self.lines, header_index, BodyBoundary::HeaderIndent);

        while cursor.index() < self.lines.len() {
            let line = self.lines[cursor.index()];
            let index = match cursor.step(self.path, line, true, self.diagnostics) {
                BodyStep::Content { index } => index,
                BodyStep::Boundary => break,
                BodyStep::Blank | BodyStep::MixedIndent => continue,
            };

            if !matches!(classify_line(line), ClassifiedLine::Statement(_)) {
                self.diagnostics
                    .push(expected_statement_or_prose(span_for_line(
                        self.path,
                        line.number,
                        line.indent_len() + 1,
                    )));
                cursor.advance();
                continue;
            }

            let (statement, next_index) = self.lower_statement(index);
            if let Some(statement) = statement {
                statements.push(statement);
            }
            cursor.set_index(next_index);
        }

        (statements, cursor.index())
    }
}

fn trim_trailing_blank_lines(lines: &mut Vec<String>) {
    while lines.last().is_some_and(String::is_empty) {
        lines.pop();
    }
}
