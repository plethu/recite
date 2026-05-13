mod action;
mod choice;
mod line;

use recite_core::{Comment, Statement};

use crate::diagnostics::{misplaced_case, misplaced_else};
use crate::layout::{ClassifiedLine, classify_line};
use crate::markers::StatementMarker;
use crate::source::{LogicalLine, span_for_line};

use super::Lowerer;

impl Lowerer<'_, '_> {
    pub(super) fn lower_statement(&mut self, index: usize) -> (Option<Statement>, usize) {
        match classify_line(self.lines[index]) {
            ClassifiedLine::Statement(StatementMarker::Line) => {
                let (line, next_index) = self.lower_line(index);
                (Some(Statement::Line(line)), next_index)
            }
            ClassifiedLine::Statement(StatementMarker::Choice) => {
                let (choice, next_index) = self.lower_choice(index);
                (Some(Statement::Choice(choice)), next_index)
            }
            ClassifiedLine::Statement(StatementMarker::Divert) => {
                (self.lower_divert(index).map(Statement::Divert), index + 1)
            }
            ClassifiedLine::Statement(StatementMarker::Effect) => {
                (self.lower_effect(index).map(Statement::Effect), index + 1)
            }
            ClassifiedLine::Statement(StatementMarker::If) => {
                let (branch, next_index) = self.lower_if(index);
                (branch.map(Statement::If), next_index)
            }
            ClassifiedLine::Statement(StatementMarker::Match) => {
                let (branch, next_index) = self.lower_match(index);
                (branch.map(Statement::Match), next_index)
            }
            ClassifiedLine::Statement(StatementMarker::Comment) => (
                Some(Statement::Comment(self.lower_comment(self.lines[index]))),
                index + 1,
            ),
            ClassifiedLine::Statement(StatementMarker::Else) => {
                let line = self.lines[index];
                self.diagnostics.push(misplaced_else(span_for_line(
                    self.path,
                    line.number,
                    line.indent_len() + 1,
                )));
                (None, self.skip_statement_body(index))
            }
            ClassifiedLine::Statement(StatementMarker::Case) => {
                let line = self.lines[index];
                self.diagnostics.push(misplaced_case(span_for_line(
                    self.path,
                    line.number,
                    line.indent_len() + 1,
                )));
                (None, self.skip_statement_body(index))
            }
            ClassifiedLine::Statement(StatementMarker::Block)
            | ClassifiedLine::Blank
            | ClassifiedLine::Prose
            | ClassifiedLine::Error => (None, index + 1),
        }
    }

    fn lower_comment(&self, line: LogicalLine<'_>) -> Comment {
        let indent = line.indent_len();
        let trimmed = line.trimmed_content();
        let text = trimmed
            .strip_prefix(StatementMarker::Comment.text())
            .expect("comment lowering only receives comment lines")
            .trim_start_matches([' ', '\t']);

        Comment::new(text, span_for_line(self.path, line.number, indent + 1))
    }

    pub(super) fn skip_statement_body(&self, header_index: usize) -> usize {
        let header_indent = self.lines[header_index].indent_len();
        let mut index = header_index + 1;

        while index < self.lines.len() {
            let line = self.lines[index];

            if classify_line(line) == ClassifiedLine::Statement(StatementMarker::Block) {
                break;
            }

            if !line.trimmed_content().is_empty() && line.indent_len() <= header_indent {
                break;
            }

            index += 1;
        }

        index
    }
}
