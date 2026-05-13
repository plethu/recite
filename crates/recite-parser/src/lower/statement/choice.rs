use recite_core::{Choice, ChoiceId, ConditionExpression, SourceText, Statement};

use crate::condition::parse_condition_expression;
use crate::diagnostics::{malformed_condition, missing_choice_id};
use crate::header::HeaderField;
use crate::markers::StatementMarker;
use crate::source::span_for_line;

use super::super::{Lowerer, header_fields};

impl Lowerer<'_, '_> {
    pub(super) fn lower_choice(&mut self, choice_index: usize) -> (Choice, usize) {
        let header = self.lines[choice_index];
        let indent = header.indent_len();
        let trimmed = header.trimmed_content();
        let base_column = indent + 1;
        let choice_span = span_for_line(self.path, header.number, base_column);
        let fields = header_fields(trimmed, StatementMarker::Choice, header, base_column);
        let if_index = fields.iter().position(|field| field.text == "if");
        let header_fields = if let Some(if_index) = if_index {
            &fields[..if_index]
        } else {
            fields.as_slice()
        };

        let mut field_start = 0;
        let choice_id = if let Some(first) = header_fields.first().copied() {
            if first.key_value(self.path).is_none() {
                field_start = 1;
                ChoiceId::new(first.text).ok()
            } else {
                None
            }
        } else {
            None
        };

        if choice_id.is_none() {
            self.diagnostics
                .push(missing_choice_id(choice_span.clone()));
        }

        let (metadata, echo) = self.lower_choice_metadata(&header_fields[field_start..]);
        let condition = if let Some(if_index) = if_index {
            self.lower_choice_condition(trimmed, base_column, fields[if_index])
        } else {
            None
        };
        let body = self.lower_prose_body(choice_index, true);
        let mut target = None;
        let mut statements = Vec::new();
        for statement in body.statements {
            if target.is_none() {
                if let Statement::Divert(divert) = statement {
                    target = Some(divert.target);
                    continue;
                }
            }
            statements.push(statement);
        }

        let mut choice = Choice::new(
            choice_id,
            SourceText::new(body.text, body.text_span),
            choice_span,
        )
        .with_metadata(metadata)
        .with_echo(echo)
        .with_statements(statements);
        if let Some(condition) = condition {
            choice = choice.with_condition(condition);
        }
        if let Some(target) = target {
            choice = choice.with_target(target);
        }

        (choice, body.next_index)
    }

    fn lower_choice_condition(
        &mut self,
        trimmed: &str,
        base_column: usize,
        field: HeaderField<'_>,
    ) -> Option<ConditionExpression> {
        let rest_start = field.offset + field.text.len();
        let rest = &trimmed[rest_start..];
        let whitespace_len = rest.len() - rest.trim_start_matches([' ', '\t']).len();
        let condition = &rest[whitespace_len..];
        let column = base_column + trimmed[..rest_start + whitespace_len].chars().count();

        match parse_condition_expression(self.path, field.line, column, condition) {
            Ok(condition) => Some(condition),
            Err(error) => {
                self.diagnostics
                    .push(malformed_condition(error.span, error.message));
                None
            }
        }
    }
}
