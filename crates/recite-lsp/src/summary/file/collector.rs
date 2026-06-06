use recite_core::{
    Block, Choice, ConditionCall, ConditionExpression, DivertTarget, Effect, IfBranch, Line,
    MatchBranch, SourceFile, SourceMetadata, SourcePosition, SourceSpan, Statement,
};

use super::{
    BlockReferenceSummary, FunctionReferenceSummary, MetadataKeySummary, MissingIdKind,
    MissingIdSummary, SpannedName,
};

#[derive(Default)]
pub(super) struct FileSummaryCollector {
    pub(super) blocks: Vec<SpannedName>,
    pub(super) block_references: Vec<BlockReferenceSummary>,
    pub(super) line_ids: Vec<SpannedName>,
    pub(super) choice_ids: Vec<SpannedName>,
    pub(super) missing_ids: Vec<MissingIdSummary>,
    pub(super) metadata_keys: Vec<MetadataKeySummary>,
    pub(super) condition_functions: Vec<FunctionReferenceSummary>,
    pub(super) effect_functions: Vec<FunctionReferenceSummary>,
}

impl FileSummaryCollector {
    pub(super) fn new() -> Self {
        Self::default()
    }

    pub(super) fn collect_source_file(&mut self, source_file: &SourceFile) {
        for block in &source_file.blocks {
            self.collect_block(block);
        }
    }

    fn collect_block(&mut self, block: &Block) {
        self.blocks.push(SpannedName {
            name: block.id.as_str().to_owned(),
            span: block.span.clone(),
        });
        self.collect_metadata(&block.metadata);
        for statement in &block.statements {
            self.collect_statement(statement);
        }
    }

    fn collect_statement(&mut self, statement: &Statement) {
        match statement {
            Statement::Line(line) => self.collect_line(line),
            Statement::Choice(choice) => self.collect_choice(choice),
            Statement::Divert(divert) => self.collect_divert_target(&divert.target, &divert.span),
            Statement::If(branch) => self.collect_if_branch(branch),
            Statement::Match(branch) => self.collect_match_branch(branch),
            Statement::Effect(effect) => self.collect_effect(effect),
            Statement::Comment(_) => {}
        }
    }

    fn collect_line(&mut self, line: &Line) {
        match &line.id {
            Some(id) => self.line_ids.push(SpannedName {
                name: id.as_str().to_owned(),
                span: line.span.clone(),
            }),
            None => self.missing_ids.push(MissingIdSummary {
                kind: MissingIdKind::Line,
                span: line.span.clone(),
                insertion_position: insertion_position_after_marker(&line.span),
            }),
        }
        self.collect_metadata(&line.metadata);
        for statement in &line.statements {
            self.collect_statement(statement);
        }
    }

    fn collect_choice(&mut self, choice: &Choice) {
        match &choice.id {
            Some(id) => self.choice_ids.push(SpannedName {
                name: id.as_str().to_owned(),
                span: choice.span.clone(),
            }),
            None => self.missing_ids.push(MissingIdSummary {
                kind: MissingIdKind::Choice,
                span: choice.span.clone(),
                insertion_position: insertion_position_after_marker(&choice.span),
            }),
        }
        self.collect_metadata(&choice.metadata);
        if let Some(requirement) = &choice.availability_requirement {
            self.collect_condition_expression(&requirement.condition);
        }
        if let Some(target) = &choice.target {
            self.collect_divert_target(&target.target, &target.span);
        }
        for statement in &choice.statements {
            self.collect_statement(statement);
        }
    }

    fn collect_if_branch(&mut self, branch: &IfBranch) {
        self.collect_condition_expression(&branch.condition);
        for statement in &branch.then_statements {
            self.collect_statement(statement);
        }
        for statement in &branch.else_statements {
            self.collect_statement(statement);
        }
    }

    fn collect_match_branch(&mut self, branch: &MatchBranch) {
        self.collect_condition_call(&branch.scrutinee);
        for arm in &branch.arms {
            for statement in &arm.statements {
                self.collect_statement(statement);
            }
        }
    }

    fn collect_effect(&mut self, effect: &Effect) {
        self.effect_functions.push(FunctionReferenceSummary {
            name: effect.function.clone(),
            span: effect
                .function_span
                .clone()
                .unwrap_or_else(|| effect.span.clone()),
        });
    }

    fn collect_divert_target(&mut self, target: &DivertTarget, span: &SourceSpan) {
        if let DivertTarget::Block(reference) = target {
            self.block_references.push(BlockReferenceSummary {
                file: reference.file.clone(),
                block_id: reference.block_id.as_str().to_owned(),
                span: span.clone(),
            });
        }
    }

    fn collect_condition_expression(&mut self, expression: &ConditionExpression) {
        match expression {
            ConditionExpression::Call(call) => self.collect_condition_call(call),
            ConditionExpression::And(group) | ConditionExpression::Or(group) => {
                for expression in &group.expressions {
                    self.collect_condition_expression(expression);
                }
            }
            ConditionExpression::Not(unary) | ConditionExpression::Grouped(unary) => {
                self.collect_condition_expression(&unary.expression);
            }
        }
    }

    fn collect_condition_call(&mut self, call: &ConditionCall) {
        self.condition_functions.push(FunctionReferenceSummary {
            name: call.function.clone(),
            span: call
                .function_span
                .clone()
                .unwrap_or_else(|| call.span.clone()),
        });
    }

    fn collect_metadata(&mut self, metadata: &SourceMetadata) {
        for entry in metadata {
            self.metadata_keys.push(MetadataKeySummary {
                key: entry.key.clone(),
                key_span: entry.key_span.clone(),
                entry_span: entry.source_span.clone(),
            });
        }
    }
}

fn insertion_position_after_marker(span: &SourceSpan) -> SourcePosition {
    SourcePosition::new(span.start.line(), span.start.column().saturating_add(1))
        .unwrap_or(span.start)
}
