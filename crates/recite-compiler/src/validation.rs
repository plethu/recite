mod project;

use std::collections::{BTreeMap, BTreeSet};

use recite_core::{
    Block, BlockReference, Choice, ConditionCall, ConditionExpression, Diagnostic, Divert,
    DivertTarget, Effect, IfBranch, Line, MatchArm, MatchBranch, Metadata, SourceFile, SourceSpan,
    SourceText, Statement,
};

use self::project::{
    collect_blocks, first_source_span, sort_diagnostics_by_source, source_files_in_project_order,
};
use crate::diagnostics;

/// Result of semantic validation over one or more Recite source files.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ValidationReport {
    pub diagnostics: Vec<Diagnostic>,
}

impl ValidationReport {
    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

/// Validate one parsed source file.
#[must_use]
pub fn validate_source_file(source_file: &SourceFile) -> ValidationReport {
    validate_source_files(std::slice::from_ref(source_file))
}

/// Validate parsed source files as one project.
#[must_use]
pub fn validate_source_files(source_files: &[SourceFile]) -> ValidationReport {
    let mut validator = Validator::new(source_files);
    validator.validate();
    sort_diagnostics_by_source(&mut validator.diagnostics);

    ValidationReport {
        diagnostics: validator.diagnostics,
    }
}

struct Validator<'a> {
    source_files: Vec<&'a SourceFile>,
    diagnostics: Vec<Diagnostic>,
    blocks: BTreeMap<&'a str, BTreeSet<&'a str>>,
    block_ids: BTreeMap<(&'a str, &'a str), SourceSpan>,
    localisable_ids: BTreeMap<&'a str, SourceSpan>,
    first_default: Option<&'a Block>,
    default_count: usize,
}

impl<'a> Validator<'a> {
    fn new(source_files: &'a [SourceFile]) -> Self {
        let source_files = source_files_in_project_order(source_files);
        let blocks = collect_blocks(&source_files);

        Self {
            source_files,
            diagnostics: Vec::new(),
            blocks,
            block_ids: BTreeMap::new(),
            localisable_ids: BTreeMap::new(),
            first_default: None,
            default_count: 0,
        }
    }

    fn validate(&mut self) {
        for source_file in self.source_files.clone() {
            self.validate_source_file(source_file);
        }

        if self.default_count == 0 && !self.source_files.is_empty() {
            self.diagnostics
                .push(diagnostics::missing_default_block(first_source_span(
                    &self.source_files,
                )));
        }
    }

    fn validate_source_file(&mut self, source_file: &'a SourceFile) {
        for block in &source_file.blocks {
            self.validate_block(source_file, block);
            for statement in &block.statements {
                self.validate_statement(source_file, statement);
            }
        }
    }

    fn validate_block(&mut self, source_file: &'a SourceFile, block: &'a Block) {
        self.validate_span(source_file, &block.span, "block");
        self.validate_metadata(source_file, &block.metadata);
        self.validate_block_id(source_file, block);
        self.validate_default_block(block);
    }

    fn validate_block_id(&mut self, source_file: &'a SourceFile, block: &'a Block) {
        let key = (source_file.path.as_str(), block.id.as_str());
        if let Some(first_span) = self.block_ids.get(&key) {
            self.diagnostics.push(diagnostics::duplicate_block_id(
                &block.id,
                block.span.clone(),
                first_span.clone(),
            ));
        } else {
            self.block_ids.insert(key, block.span.clone());
        }
    }

    fn validate_default_block(&mut self, block: &'a Block) {
        if !block.is_default {
            return;
        }

        self.default_count += 1;
        if let Some(first) = self.first_default {
            self.diagnostics
                .push(diagnostics::ambiguous_default_block(block, first));
        } else {
            self.first_default = Some(block);
        }
    }

    fn validate_statement(&mut self, source_file: &'a SourceFile, statement: &'a Statement) {
        match statement {
            Statement::Line(line) => {
                self.validate_line(source_file, line);
                for statement in &line.statements {
                    self.validate_statement(source_file, statement);
                }
            }
            Statement::Choice(choice) => {
                self.validate_choice(source_file, choice);
                for statement in &choice.statements {
                    self.validate_statement(source_file, statement);
                }
            }
            Statement::Divert(divert) => self.validate_divert(source_file, divert),
            Statement::If(branch) => {
                self.validate_if_branch(source_file, branch);
                for statement in &branch.then_statements {
                    self.validate_statement(source_file, statement);
                }
                for statement in &branch.else_statements {
                    self.validate_statement(source_file, statement);
                }
            }
            Statement::Match(branch) => {
                self.validate_match_branch(source_file, branch);
                for arm in &branch.arms {
                    self.validate_match_arm(source_file, arm);
                    for statement in &arm.statements {
                        self.validate_statement(source_file, statement);
                    }
                }
            }
            Statement::Effect(effect) => self.validate_effect(source_file, effect),
            Statement::Comment(comment) => {
                self.validate_span(source_file, &comment.span, "comment");
            }
        }
    }

    fn validate_line(&mut self, source_file: &'a SourceFile, line: &'a Line) {
        self.validate_span(source_file, &line.span, "line");
        self.validate_source_text(source_file, &line.source_text, "line source text");
        self.validate_metadata(source_file, &line.metadata);

        let Some(id) = &line.id else {
            self.diagnostics.push(diagnostics::missing_line_id(line));
            return;
        };

        if let Some(first_span) = self.localisable_ids.get(id.as_str()) {
            self.diagnostics
                .push(diagnostics::duplicate_line_id(line, first_span.clone()));
        } else {
            self.localisable_ids.insert(id.as_str(), line.span.clone());
        }
    }

    fn validate_choice(&mut self, source_file: &'a SourceFile, choice: &'a Choice) {
        self.validate_span(source_file, &choice.span, "choice");
        self.validate_source_text(source_file, &choice.source_text, "choice source text");
        self.validate_metadata(source_file, &choice.metadata);
        if let Some(condition) = &choice.condition {
            self.validate_condition_expression(source_file, condition);
        }

        if let Some(id) = &choice.id {
            if let Some(first_span) = self.localisable_ids.get(id.as_str()) {
                self.diagnostics
                    .push(diagnostics::duplicate_choice_id(choice, first_span.clone()));
            } else {
                self.localisable_ids
                    .insert(id.as_str(), choice.span.clone());
            }
        } else {
            self.diagnostics
                .push(diagnostics::missing_choice_id(choice));
        }

        if let Some(target) = &choice.target {
            self.validate_span(source_file, &target.span, "choice target");
            self.validate_reference(source_file, &target.target, &target.span);
        }
    }

    fn validate_divert(&mut self, source_file: &'a SourceFile, divert: &'a Divert) {
        self.validate_span(source_file, &divert.span, "divert");
        self.validate_reference(source_file, &divert.target, &divert.span);
    }

    fn validate_if_branch(&mut self, source_file: &'a SourceFile, branch: &'a IfBranch) {
        self.validate_span(source_file, &branch.span, "if branch");
        self.validate_condition_expression(source_file, &branch.condition);
    }

    fn validate_match_branch(&mut self, source_file: &'a SourceFile, branch: &'a MatchBranch) {
        self.validate_span(source_file, &branch.span, "match branch");
        self.validate_condition_call(source_file, &branch.scrutinee);
    }

    fn validate_match_arm(&mut self, source_file: &'a SourceFile, arm: &'a MatchArm) {
        self.validate_span(source_file, &arm.span, "match arm");
    }

    fn validate_effect(&mut self, source_file: &'a SourceFile, effect: &'a Effect) {
        self.validate_span(source_file, &effect.span, "effect");
    }

    fn validate_source_text(
        &mut self,
        source_file: &'a SourceFile,
        source_text: &'a SourceText,
        owner: &'static str,
    ) {
        self.validate_span(source_file, &source_text.span, owner);
    }

    fn validate_metadata(&mut self, source_file: &'a SourceFile, metadata: &'a Metadata) {
        for entry in metadata {
            if let Some(span) = &entry.source_span {
                self.validate_span(source_file, span, "metadata entry");
            }
            if let Some(span) = &entry.key_span {
                self.validate_span(source_file, span, "metadata key");
            }
            if let Some(span) = &entry.value_span {
                self.validate_span(source_file, span, "metadata value");
            }
        }
    }

    fn validate_condition_expression(
        &mut self,
        source_file: &'a SourceFile,
        condition: &'a ConditionExpression,
    ) {
        match condition {
            ConditionExpression::Call(call) => self.validate_condition_call(source_file, call),
            ConditionExpression::And(group) | ConditionExpression::Or(group) => {
                self.validate_span(source_file, &group.span, "condition expression");
                for expression in &group.expressions {
                    self.validate_condition_expression(source_file, expression);
                }
            }
            ConditionExpression::Not(unary) | ConditionExpression::Grouped(unary) => {
                self.validate_span(source_file, &unary.span, "condition expression");
                self.validate_condition_expression(source_file, &unary.expression);
            }
        }
    }

    fn validate_condition_call(&mut self, source_file: &'a SourceFile, call: &'a ConditionCall) {
        self.validate_span(source_file, &call.span, "condition call");
    }

    fn validate_span(
        &mut self,
        source_file: &'a SourceFile,
        span: &SourceSpan,
        owner: &'static str,
    ) {
        if span.file != source_file.path {
            self.diagnostics.push(diagnostics::invalid_source_span(
                span.clone(),
                owner,
                "span file does not match source file",
            ));
        }

        if span.end.is_some_and(|end| end < span.start) {
            self.diagnostics.push(diagnostics::invalid_source_span(
                span.clone(),
                owner,
                "span end precedes span start",
            ));
        }
    }

    fn validate_reference(
        &mut self,
        source_file: &'a SourceFile,
        target: &'a DivertTarget,
        span: &SourceSpan,
    ) {
        let DivertTarget::Block(reference) = target else {
            return;
        };

        if !self.contains_block_reference(source_file, reference) {
            self.diagnostics.push(diagnostics::unknown_block_reference(
                reference,
                span.clone(),
            ));
        }
    }

    fn contains_block_reference(
        &self,
        source_file: &'a SourceFile,
        reference: &'a BlockReference,
    ) -> bool {
        let file = reference
            .file
            .as_deref()
            .unwrap_or(source_file.path.as_str());
        let Some(blocks) = self.blocks.get(file) else {
            return false;
        };

        blocks.contains(reference.block_id.as_str())
    }
}
