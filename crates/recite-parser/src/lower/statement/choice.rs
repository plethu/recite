use recite_core::{
    AvailabilityReasonId, Choice, ChoiceAvailabilityReasonOverride, ChoiceAvailabilityRequirement,
    ChoiceId, ChoiceTarget, SourceSpan, SourceText, Statement,
};

use crate::condition::parse_condition_expression;
use crate::diagnostics::{malformed_condition, malformed_header, trailing_choice_if};
use crate::header::{HeaderField, HeaderKeyValue};
use crate::markers::StatementMarker;
use crate::source::{span_for_line, span_for_text};

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
        if let Some(if_index) = if_index {
            self.diagnostics
                .push(trailing_choice_if(fields[if_index].span(self.path)));
        }
        let choice_fields = if let Some(if_index) = if_index {
            &fields[..if_index]
        } else {
            fields.as_slice()
        };

        let mut field_start = 0;
        let choice_id = if let Some(first) = choice_fields.first().copied() {
            if first.key_value(self.path).is_none() {
                field_start = 1;
                ChoiceId::new(first.text).ok()
            } else {
                None
            }
        } else {
            None
        };

        let ChoiceClauses {
            metadata_fields,
            availability_requirement,
            availability_reason_override,
        } = self.lower_choice_clauses(&choice_fields[field_start..]);
        let (metadata, echo) = self.lower_choice_metadata(&metadata_fields);
        let body = self.lower_prose_body(choice_index, true);
        let mut target = None;
        let mut statements = Vec::new();
        for statement in body.statements {
            match statement {
                Statement::Divert(divert) if target.is_none() => {
                    target = Some(ChoiceTarget::new(divert.target, divert.span));
                }
                statement => statements.push(statement),
            }
        }

        let mut choice = Choice::new(
            choice_id,
            SourceText::new(body.text, body.text_span),
            choice_span,
        )
        .with_metadata(metadata)
        .with_echo(echo)
        .with_statements(statements);
        if let Some(requirement) = availability_requirement {
            choice = choice.with_availability_requirement(requirement);
        }
        if let Some(reason) = availability_reason_override {
            choice = choice.with_availability_reason_override(reason);
        }
        if let Some(target) = target {
            choice = choice.with_target(target);
        }

        (choice, body.next_index)
    }

    fn lower_choice_clauses<'a>(&mut self, fields: &[HeaderField<'a>]) -> ChoiceClauses<'a> {
        let mut metadata_fields = Vec::new();
        let mut availability_requirement = None;
        let mut availability_reason_override = None;

        for field in fields.iter().copied() {
            let Some(kv) = field.key_value(self.path) else {
                metadata_fields.push(field);
                continue;
            };

            match kv.key {
                "requires" => {
                    if availability_requirement.is_some() {
                        self.diagnostics.push(malformed_header(kv.key_span));
                        continue;
                    }
                    availability_requirement = self.lower_availability_requirement(&kv);
                }
                "reason" => {
                    if availability_reason_override.is_some() {
                        self.diagnostics.push(malformed_header(kv.key_span));
                        continue;
                    }
                    availability_reason_override = self.lower_availability_reason_override(&kv);
                }
                _ => metadata_fields.push(field),
            }
        }

        ChoiceClauses {
            metadata_fields,
            availability_requirement,
            availability_reason_override,
        }
    }

    fn lower_availability_requirement(
        &mut self,
        kv: &HeaderKeyValue<'_>,
    ) -> Option<ChoiceAvailabilityRequirement> {
        match parse_condition_expression(
            self.path,
            kv.value_span.start.line(),
            source_column(kv.value_span.start.column()),
            kv.value,
        ) {
            Ok(condition) => Some(ChoiceAvailabilityRequirement::new(
                condition,
                kv.field_span.clone(),
            )),
            Err(error) => {
                self.diagnostics
                    .push(malformed_condition(error.span, error.message));
                None
            }
        }
    }

    fn lower_availability_reason_override(
        &mut self,
        kv: &HeaderKeyValue<'_>,
    ) -> Option<ChoiceAvailabilityReasonOverride> {
        let Some(parsed) = parse_reason_override_value(self.path, kv) else {
            self.diagnostics
                .push(malformed_header(kv.value_span.clone()));
            return None;
        };
        let Ok(reason_id) = AvailabilityReasonId::new(parsed.id) else {
            self.diagnostics.push(malformed_header(parsed.id_span));
            return None;
        };

        let mut reason =
            ChoiceAvailabilityReasonOverride::new(reason_id, kv.field_span.clone(), parsed.id_span);
        if let Some(argument_span) = parsed.argument_span {
            reason = reason.with_argument_span(argument_span);
        }
        Some(reason)
    }
}

struct ChoiceClauses<'a> {
    metadata_fields: Vec<HeaderField<'a>>,
    availability_requirement: Option<ChoiceAvailabilityRequirement>,
    availability_reason_override: Option<ChoiceAvailabilityReasonOverride>,
}

struct ParsedReasonOverride<'a> {
    id: &'a str,
    id_span: SourceSpan,
    argument_span: Option<SourceSpan>,
}

fn parse_reason_override_value<'a>(
    path: &str,
    kv: &HeaderKeyValue<'a>,
) -> Option<ParsedReasonOverride<'a>> {
    if let Some(open) = kv.value.find('(') {
        let close = kv.value.rfind(')')?;
        if close != kv.value.len() - 1 || open == 0 {
            return None;
        }
        let id = &kv.value[..open];
        let argument_text = &kv.value[open..=close];
        let value_column = source_column(kv.value_span.start.column());
        let id_span = span_for_text(path, kv.value_span.start.line(), value_column, id);
        let argument_span = span_for_text(
            path,
            kv.value_span.start.line(),
            value_column + id.chars().count(),
            argument_text,
        );
        return Some(ParsedReasonOverride {
            id,
            id_span,
            argument_span: Some(argument_span),
        });
    }

    if kv.value.contains(')') {
        return None;
    }

    Some(ParsedReasonOverride {
        id: kv.value,
        id_span: kv.value_span.clone(),
        argument_span: None,
    })
}

fn source_column(column: u32) -> usize {
    column as usize
}
