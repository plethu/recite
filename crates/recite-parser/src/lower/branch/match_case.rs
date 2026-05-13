use recite_core::{MatchArm, MatchBranch, MatchPattern};

use crate::body::{BodyBoundary, BodyCursor, BodyStep};
use crate::diagnostics::malformed_case;
use crate::layout::{ClassifiedLine, classify_line};
use crate::markers::StatementMarker;
use crate::source::span_for_line;

use super::super::{Lowerer, header_fields};
use super::helpers::{directive_header, parse_condition_call_header};

impl Lowerer<'_, '_> {
    pub(in crate::lower) fn lower_match(&mut self, index: usize) -> (Option<MatchBranch>, usize) {
        let line = self.lines[index];
        let header = directive_header(self.path, line, StatementMarker::Match);
        let scrutinee = parse_condition_call_header(self.path, self.diagnostics, &header);
        let (arms, next_index) = self.lower_match_arms(index);

        let Some(scrutinee) = scrutinee else {
            return (None, next_index);
        };

        (
            Some(MatchBranch::new(scrutinee, arms, header.span)),
            next_index,
        )
    }

    fn lower_match_arms(&mut self, header_index: usize) -> (Vec<MatchArm>, usize) {
        let mut arms = Vec::new();
        let mut cursor = BodyCursor::new(&self.lines, header_index, BodyBoundary::HeaderIndent);

        while cursor.index() < self.lines.len() {
            let line = self.lines[cursor.index()];
            let index = match cursor.step(self.path, line, true, self.diagnostics) {
                BodyStep::Content { index } => index,
                BodyStep::Boundary => break,
                BodyStep::Blank | BodyStep::MixedIndent => continue,
            };

            if classify_line(line) != ClassifiedLine::Statement(StatementMarker::Case) {
                self.diagnostics.push(malformed_case(span_for_line(
                    self.path,
                    line.number,
                    line.indent_len() + 1,
                )));
                let next_index = if matches!(classify_line(line), ClassifiedLine::Statement(_)) {
                    self.skip_statement_body(index)
                } else {
                    index + 1
                };
                cursor.set_index(next_index);
                continue;
            }

            let (arm, next_index) = self.lower_case(index);
            if let Some(arm) = arm {
                arms.push(arm);
            }
            cursor.set_index(next_index);
        }

        (arms, cursor.index())
    }

    fn lower_case(&mut self, index: usize) -> (Option<MatchArm>, usize) {
        let line = self.lines[index];
        let indent = line.indent_len();
        let trimmed = line.trimmed_content();
        let base_column = indent + 1;
        let fields = header_fields(trimmed, StatementMarker::Case, line, base_column);
        let (statements, next_index) = self.lower_statement_body(index);

        let Some(pattern_field) = fields.first().copied() else {
            self.diagnostics.push(malformed_case(span_for_line(
                self.path,
                line.number,
                base_column,
            )));
            return (None, next_index);
        };

        if fields.len() > 1 {
            self.diagnostics
                .push(malformed_case(fields[1].span(self.path)));
            return (None, next_index);
        }

        let pattern = if pattern_field.text == "_" {
            MatchPattern::Wildcard
        } else {
            MatchPattern::Variant(pattern_field.text.to_owned())
        };

        (
            Some(MatchArm::new(
                pattern,
                statements,
                span_for_line(self.path, line.number, base_column),
            )),
            next_index,
        )
    }
}
