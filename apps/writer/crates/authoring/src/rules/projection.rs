use super::{ReplyRules, RuleArgument, RuleEffect, RuleExpression};
use crate::{Document, EditError, projection::offset};
use recite_core::{Choice, ConditionExpression, SourceId, Statement};

impl Document {
    pub fn reply_rules(&self, id: &str) -> Result<ReplyRules, EditError> {
        let parsed = recite_parser::parse(self.key().as_str(), self.source()).lower_source_file();
        if !parsed.diagnostics.is_empty() {
            return Err(EditError::SourceRequired("repair the syntax diagnostics"));
        }
        let mut found = Vec::new();
        for block in &parsed.source_file.blocks {
            for statement in &block.statements {
                statement.visit_depth_first(&mut |statement| {
                    if let Statement::Choice(choice) = statement
                        && matches!(&choice.source_id, SourceId::Frozen { anchor, .. } if anchor.as_str() == id) {
                        found.push(choice);
                    }
                });
            }
        }
        let [choice] = found.as_slice() else {
            return Err(EditError::MissingPassage);
        };
        let destination = choice.target.as_ref().and_then(|target| {
            let recite_core::DivertTarget::Block(reference) = &target.target else {
                return None;
            };
            if reference.file.is_some() {
                return None;
            }
            let mut matches = parsed
                .source_file
                .blocks
                .iter()
                .filter(|b| b.id == reference.block_id);
            let block = matches.next()?;
            matches.next().is_none().then_some(block)
        });
        self.project_reply(id, choice, destination)
    }
    fn project_reply(
        &self,
        id: &str,
        choice: &Choice,
        destination: Option<&recite_core::Block>,
    ) -> Result<ReplyRules, EditError> {
        let mut effects = Vec::new();
        let mut ranges = Vec::new();
        let mut has_other_statements = choice
            .statements
            .iter()
            .any(|s| !matches!(s, Statement::Comment(_)));
        let statements = destination.map_or(&[][..], |block| block.statements.as_slice());
        for (index, statement) in statements.iter().enumerate() {
            match statement {
                Statement::Effect(effect) => {
                    if effect.args.len() != effect.arg_spans.len() {
                        return Err(EditError::Position);
                    }
                    let definition = self.schema().and_then(|s| s.effects.get(&effect.function));
                    let range = offset(self.source(), effect.span.start)?
                        ..super::span_range(
                            self.source(),
                            effect.call_span.as_ref().ok_or(EditError::Position)?,
                        )?
                        .end;
                    effects.push(RuleEffect {
                        function: effect.function.clone(),
                        mode: effect.mode,
                        allowed_modes: definition.map_or_else(
                            || vec![effect.mode],
                            |d| d.modes.iter().copied().collect(),
                        ),
                        arguments: effect
                            .args
                            .iter()
                            .enumerate()
                            .map(|(i, a)| {
                                RuleArgument::new(
                                    a,
                                    definition.and_then(|d| d.params.get(i)),
                                    self.schema(),
                                    i,
                                )
                            })
                            .collect(),
                        original: Some(effect.clone()),
                        raw: self
                            .source()
                            .get(range.clone())
                            .ok_or(EditError::Position)?
                            .into(),
                    });
                    ranges.push(range);
                }
                Statement::Comment(_) => {}
                Statement::Divert(_) if index + 1 == statements.len() => {}
                _ => has_other_statements = true,
            }
        }
        let condition = choice
            .availability_requirement
            .as_ref()
            .map(|r| project_condition(&r.condition, self.schema()));
        let condition_range = choice
            .availability_requirement
            .as_ref()
            .map(|r| super::span_range(self.source(), r.condition.span()))
            .transpose()?;
        let requirement_range = choice
            .availability_requirement
            .as_ref()
            .map(|r| super::span_range(self.source(), &r.span))
            .transpose()?;
        let effect_insertion = statements
            .last()
            .and_then(|s| match s {
                Statement::Divert(divert) => Some(&divert.span),
                _ => None,
            })
            .map(|span| {
                let start = offset(
                    self.source(),
                    recite_core::SourcePosition::new(span.start.line(), 1)?,
                )?;
                let end = offset(self.source(), span.start)?;
                Ok::<_, EditError>((start, self.source()[start..end].to_owned()))
            })
            .transpose()?;
        let available_conditions = self
            .schema()
            .map(|schema| {
                schema
                    .conditions
                    .iter()
                    .filter(|(_, d)| d.returns == recite_core::ConditionReturnType::Bool)
                    .map(|(name, d)| RuleExpression::Call {
                        function: name.clone(),
                        arguments: d
                            .params
                            .iter()
                            .map(|p| super::values::default_argument(p, schema))
                            .collect(),
                    })
                    .collect()
            })
            .unwrap_or_default();
        let available_effects = self
            .schema()
            .map(|schema| {
                schema
                    .effects
                    .iter()
                    .filter_map(|(name, d)| {
                        let mode = d.modes.iter().next().copied()?;
                        Some(RuleEffect {
                            function: name.clone(),
                            arguments: d
                                .params
                                .iter()
                                .map(|p| super::values::default_argument(p, schema))
                                .collect(),
                            mode,
                            allowed_modes: d.modes.iter().copied().collect(),
                            original: None,
                            raw: String::new(),
                        })
                    })
                    .collect()
            })
            .unwrap_or_default();
        let header = self
            .source()
            .lines()
            .nth((choice.span.start.line() - 1) as usize)
            .ok_or(EditError::Position)?;
        let header_start = offset(
            self.source(),
            recite_core::SourcePosition::new(choice.span.start.line(), 1)?,
        )?;
        let text = self
            .find_passage(id)?
            .ok_or(EditError::MissingPassage)?
            .text;
        Ok(ReplyRules {
            passage: id.into(),
            destination: destination.map(|block| block.id.to_string()),
            text,
            original_condition: condition.clone(),
            condition,
            effects,
            effect_order_editable: !has_other_statements && effect_insertion.is_some(),
            has_other_statements,
            available_conditions,
            available_effects,
            requirement_range,
            effect_insertion,
            key: self.key().clone(),
            revision: self.revision(),
            source: self.source().into(),
            condition_range,
            header_end: header_start + header.trim_end_matches('\r').len(),
            effect_ranges: ranges,
        })
    }
}
fn project_condition(
    condition: &ConditionExpression,
    schema: Option<&recite_core::ProjectSchema>,
) -> RuleExpression {
    match condition {
        ConditionExpression::Call(call) => RuleExpression::Call {
            function: call.function.clone(),
            arguments: call
                .args
                .iter()
                .enumerate()
                .map(|(i, a)| {
                    RuleArgument::new(
                        a,
                        schema
                            .and_then(|s| s.conditions.get(&call.function))
                            .and_then(|d| d.params.get(i)),
                        schema,
                        i,
                    )
                })
                .collect(),
        },
        ConditionExpression::And(group) => RuleExpression::All(
            group
                .expressions
                .iter()
                .map(|c| project_condition(c, schema))
                .collect(),
        ),
        ConditionExpression::Or(group) => RuleExpression::Any(
            group
                .expressions
                .iter()
                .map(|c| project_condition(c, schema))
                .collect(),
        ),
        ConditionExpression::Not(inner) => {
            RuleExpression::Not(Box::new(project_condition(&inner.expression, schema)))
        }
        ConditionExpression::Grouped(inner) => {
            RuleExpression::Group(Box::new(project_condition(&inner.expression, schema)))
        }
    }
}
