use recite_core::{
    BlockId, SourceSpan,
    ast::{
        Block, BlockReference, Choice, ConditionCall, ConditionExpression, DivertTarget, Effect,
        EffectMode, IfBranch, InterpolationBinding, Line, MatchBranch, SourceFile, SourceMetadata,
        Statement,
    },
};
use std::{collections::BTreeSet, sync::Arc};

mod regions;
#[path = "types.rs"]
mod types;
pub(crate) use regions::SummaryRanges;
pub use types::*;

/// A deterministic, host-neutral summary of one lowered source file.
#[derive(Clone, Debug, Default, PartialEq)]
#[non_exhaustive]
pub struct AuthoringSummary {
    blocks: Arc<Vec<BlockDefinitionSummary>>,
    block_references: Arc<Vec<BlockReferenceSummary>>,
    stable_ids: Arc<Vec<StableIdSummary>>,
    metadata: Arc<Vec<MetadataSummary>>,
    condition_functions: Arc<Vec<FunctionReferenceSummary>>,
    effect_functions: Arc<Vec<FunctionReferenceSummary>>,
    bindings: Arc<Vec<InterpolationBinding>>,
}

impl AuthoringSummary {
    pub(crate) fn from_source_file(source_file: &SourceFile) -> Self {
        let mut collector = Collector::default();
        for block in &source_file.blocks {
            collector.block(block);
        }
        Self {
            blocks: Arc::new(collector.blocks),
            block_references: Arc::new(collector.block_references),
            stable_ids: Arc::new(collector.stable_ids),
            metadata: Arc::new(collector.metadata),
            condition_functions: Arc::new(collector.condition_functions),
            effect_functions: Arc::new(collector.effect_functions),
            bindings: Arc::new(collector.bindings),
        }
    }

    /// First definition per caller value in each block, in source order.
    /// Regions retain their own definitions so removing an earlier region can
    /// reveal a later definition without reparsing it.
    #[must_use]
    pub fn interpolation_bindings(&self) -> &[InterpolationBinding] {
        &self.bindings
    }

    #[must_use]
    pub fn blocks(&self) -> &[BlockDefinitionSummary] {
        &self.blocks
    }

    #[must_use]
    pub fn block_references(&self) -> &[BlockReferenceSummary] {
        &self.block_references
    }

    #[must_use]
    pub fn stable_ids(&self) -> &[StableIdSummary] {
        &self.stable_ids
    }

    #[must_use]
    pub fn metadata(&self) -> &[MetadataSummary] {
        &self.metadata
    }

    #[must_use]
    pub fn condition_functions(&self) -> &[FunctionReferenceSummary] {
        &self.condition_functions
    }

    #[must_use]
    pub fn effect_functions(&self) -> &[FunctionReferenceSummary] {
        &self.effect_functions
    }
}

#[derive(Default)]
struct Collector {
    blocks: Vec<BlockDefinitionSummary>,
    block_references: Vec<BlockReferenceSummary>,
    stable_ids: Vec<StableIdSummary>,
    metadata: Vec<MetadataSummary>,
    condition_functions: Vec<FunctionReferenceSummary>,
    effect_functions: Vec<FunctionReferenceSummary>,
    bindings: Vec<InterpolationBinding>,
    binding_values: BTreeSet<String>,
}

impl Collector {
    fn block(&mut self, block: &Block) {
        self.binding_values.clear();
        self.blocks.push(BlockDefinitionSummary {
            id: block.id.clone(),
            id_span: block.id_span.clone(),
            span: block.span.clone(),
            is_default: block.is_default,
            has_end_destination: false,
        });
        self.metadata(&block.metadata);
        for statement in &block.statements {
            self.statement(&block.id, statement);
        }
    }

    fn statement(&mut self, enclosing_block: &BlockId, statement: &Statement) {
        match statement {
            Statement::Line(line) => self.line(enclosing_block, line),
            Statement::Choice(choice) => self.choice(enclosing_block, choice),
            Statement::Divert(divert) => self.divert(&divert.target, &divert.span),
            Statement::If(branch) => self.if_branch(enclosing_block, branch),
            Statement::Match(branch) => self.match_branch(enclosing_block, branch),
            Statement::Effect(effect) => self.effect(effect),
            Statement::Comment(_) => {}
        }
    }

    fn line(&mut self, enclosing_block: &BlockId, line: &Line) {
        self.bindings(&line.interpolation_bindings);
        self.stable_ids.push(StableIdSummary {
            kind: StableIdKind::Line,
            source_id: line.source_id.clone(),
            source_id_span: line.source_id_span.clone(),
            insertion_span: line.source_id_insertion_span.clone(),
            span: line.span.clone(),
            enclosing_block: enclosing_block.clone(),
        });
        self.metadata(&line.metadata);
        for statement in &line.statements {
            self.statement(enclosing_block, statement);
        }
    }

    fn choice(&mut self, enclosing_block: &BlockId, choice: &Choice) {
        self.bindings(&choice.interpolation_bindings);
        self.stable_ids.push(StableIdSummary {
            kind: StableIdKind::Choice,
            source_id: choice.source_id.clone(),
            source_id_span: choice.source_id_span.clone(),
            insertion_span: choice.source_id_insertion_span.clone(),
            span: choice.span.clone(),
            enclosing_block: enclosing_block.clone(),
        });
        self.metadata(&choice.metadata);
        if let Some(requirement) = &choice.availability_requirement {
            self.condition_expression(
                &requirement.condition,
                FunctionReferenceKind::BooleanCondition,
            );
        }
        if let Some(target) = &choice.target {
            self.divert(&target.target, &target.span);
        }
        for statement in &choice.statements {
            self.statement(enclosing_block, statement);
        }
    }

    fn divert(&mut self, target: &DivertTarget, span: &SourceSpan) {
        if matches!(target, DivertTarget::End)
            && let Some(block) = self.blocks.last_mut()
        {
            block.has_end_destination = true;
        }
        if let DivertTarget::Block(BlockReference {
            file,
            file_span,
            block_id,
            block_id_span,
            ..
        }) = target
        {
            self.block_references.push(BlockReferenceSummary {
                file: file.clone(),
                file_span: file_span.clone(),
                block_id: block_id.clone(),
                block_id_span: block_id_span.clone(),
                span: span.clone(),
            });
        }
    }

    fn if_branch(&mut self, enclosing_block: &BlockId, branch: &IfBranch) {
        self.condition_expression(&branch.condition, FunctionReferenceKind::BooleanCondition);
        for statement in &branch.then_statements {
            self.statement(enclosing_block, statement);
        }
        for statement in &branch.else_statements {
            self.statement(enclosing_block, statement);
        }
    }

    fn match_branch(&mut self, enclosing_block: &BlockId, branch: &MatchBranch) {
        self.condition_call(&branch.scrutinee, FunctionReferenceKind::MatchCondition);
        for arm in &branch.arms {
            for statement in &arm.statements {
                self.statement(enclosing_block, statement);
            }
        }
    }

    fn condition_expression(
        &mut self,
        expression: &ConditionExpression,
        kind: FunctionReferenceKind,
    ) {
        match expression {
            ConditionExpression::Call(call) => self.condition_call(call, kind),
            ConditionExpression::And(group) | ConditionExpression::Or(group) => {
                for expression in &group.expressions {
                    self.condition_expression(expression, kind);
                }
            }
            ConditionExpression::Not(unary) | ConditionExpression::Grouped(unary) => {
                self.condition_expression(&unary.expression, kind);
            }
        }
    }

    fn condition_call(&mut self, call: &ConditionCall, kind: FunctionReferenceKind) {
        self.condition_functions.push(FunctionReferenceSummary {
            name: call.function.clone(),
            span: call
                .function_span
                .clone()
                .unwrap_or_else(|| call.span.clone()),
            argument_count: call.args.len(),
            kind,
        });
    }

    fn effect(&mut self, effect: &Effect) {
        let kind = match effect.mode {
            EffectMode::Deferred => FunctionReferenceKind::DeferredEffect,
            EffectMode::Immediate => FunctionReferenceKind::ImmediateEffect,
            EffectMode::Blocking => FunctionReferenceKind::BlockingEffect,
        };
        self.effect_functions.push(FunctionReferenceSummary {
            name: effect.function.clone(),
            span: effect
                .function_span
                .clone()
                .unwrap_or_else(|| effect.span.clone()),
            argument_count: effect.args.len(),
            kind,
        });
    }

    fn bindings(&mut self, bindings: &[InterpolationBinding]) {
        for binding in bindings {
            if !self.binding_values.contains(&binding.value) {
                self.binding_values.insert(binding.value.clone());
                self.bindings.push(binding.clone());
            }
        }
    }

    fn metadata(&mut self, metadata: &SourceMetadata) {
        for entry in metadata {
            self.metadata.push(MetadataSummary {
                key: entry.key.clone(),
                source_span: entry.source_span.clone(),
                key_span: entry.key_span.clone(),
                value_span: entry.value_span.clone(),
                value_element_spans: entry.value_element_spans().to_vec(),
                value: (&entry.value).into(),
            });
        }
    }
}
