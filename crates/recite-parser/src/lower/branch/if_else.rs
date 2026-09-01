use recite_core::{IfBranch, SourceRecoveryClass, Statement};

use crate::diagnostics::malformed_header;
use crate::layout::{ClassifiedLine, classify_line};
use crate::markers::StatementMarker;
use crate::source::span_for_text;

use super::super::Lowerer;
use super::helpers::{directive_header, parse_condition_expression_header};

impl Lowerer<'_, '_> {
    pub(in crate::lower) fn lower_if(&mut self, index: usize) -> (Option<IfBranch>, usize) {
        let line = self.lines[index];
        let indent = line.indent_len();
        let header = directive_header(self.path, line, StatementMarker::If);
        let condition = parse_condition_expression_header(self.path, self.diagnostics, &header);
        if condition.is_none() {
            self.mark(SourceRecoveryClass::ConditionFunctions);
            super::super::mark_all(self.recovery);
        }
        let (then_statements, mut next_index) = self.lower_statement_body(index);
        let else_statements = if self.is_else_at(next_index, indent) {
            let (else_body, after_else) = self.lower_else(next_index);
            next_index = after_else;
            else_body
        } else {
            Vec::new()
        };

        let Some(condition) = condition else {
            return (None, next_index);
        };

        let branch = IfBranch::new(condition, then_statements, header.span)
            .with_else_statements(else_statements);

        (Some(branch), next_index)
    }

    fn lower_else(&mut self, index: usize) -> (Vec<Statement>, usize) {
        let line = self.lines[index];
        let header = directive_header(self.path, line, StatementMarker::Else);
        if !header.text.is_empty() {
            self.mark(SourceRecoveryClass::ConditionFunctions);
            self.diagnostics.push(malformed_header(span_for_text(
                self.path,
                header.line,
                header.column,
                header.text,
            )));
        }

        self.lower_statement_body(index)
    }

    fn is_else_at(&self, index: usize, indent: usize) -> bool {
        self.lines.get(index).is_some_and(|line| {
            line.indent_len() == indent
                && classify_line(*line) == ClassifiedLine::Statement(StatementMarker::Else)
        })
    }
}
