use recite_core::{
    Block, Choice, Divert, Effect, IfBranch, Line, MatchArm, MatchBranch, MetadataTarget,
    SourceFile, Statement,
};

use super::state::Validator;
use crate::diagnostics;

impl<'a> Validator<'a> {
    pub(super) fn validate_block(&mut self, source_file: &'a SourceFile, block: &'a Block) {
        self.validate_span(source_file, &block.span, "block");
        self.validate_metadata(source_file, &block.metadata, MetadataTarget::Block);
        self.validate_block_id(source_file, block);
        self.validate_default_block(block);
    }
    pub(super) fn validate_statement(
        &mut self,
        source_file: &'a SourceFile,
        statement: &'a Statement,
    ) {
        match statement {
            Statement::Line(line) => {
                self.validate_line(source_file, line);
                for statement in &line.statements {
                    if !matches!(statement, Statement::Choice(_)) {
                        self.diagnostics
                            .push(diagnostics::unsupported_line_child_statement(
                                line, statement,
                            ));
                    }
                    self.validate_statement(source_file, statement);
                }
            }
            Statement::Choice(choice) => {
                self.validate_choice(source_file, choice);
                for statement in &choice.statements {
                    self.diagnostics
                        .push(diagnostics::unsupported_choice_child_statement(
                            choice, statement,
                        ));
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
    pub(super) fn validate_line(&mut self, source_file: &'a SourceFile, line: &'a Line) {
        self.validate_span(source_file, &line.span, "line");
        self.validate_source_text(source_file, &line.source_text, "line source text");
        self.validate_metadata(source_file, &line.metadata, MetadataTarget::Line);

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
    pub(super) fn validate_choice(&mut self, source_file: &'a SourceFile, choice: &'a Choice) {
        self.validate_span(source_file, &choice.span, "choice");
        self.validate_source_text(source_file, &choice.source_text, "choice source text");
        self.validate_metadata(source_file, &choice.metadata, MetadataTarget::Choice);
        self.validate_choice_echo(choice);
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
        } else {
            self.diagnostics
                .push(diagnostics::missing_choice_target(choice));
        }
    }
    pub(super) fn validate_divert(&mut self, source_file: &'a SourceFile, divert: &'a Divert) {
        self.validate_span(source_file, &divert.span, "divert");
        self.validate_reference(source_file, &divert.target, &divert.span);
    }
    pub(super) fn validate_if_branch(&mut self, source_file: &'a SourceFile, branch: &'a IfBranch) {
        self.validate_span(source_file, &branch.span, "if branch");
        self.validate_condition_expression(source_file, &branch.condition);
    }
    pub(super) fn validate_match_branch(
        &mut self,
        source_file: &'a SourceFile,
        branch: &'a MatchBranch,
    ) {
        self.validate_span(source_file, &branch.span, "match branch");
        self.validate_condition_call(source_file, &branch.scrutinee);
    }
    pub(super) fn validate_match_arm(&mut self, source_file: &'a SourceFile, arm: &'a MatchArm) {
        self.validate_span(source_file, &arm.span, "match arm");
    }
    pub(super) fn validate_effect(&mut self, source_file: &'a SourceFile, effect: &'a Effect) {
        self.validate_span(source_file, &effect.span, "effect");
        if let Some(span) = &effect.mode_span {
            self.validate_span(source_file, span, "effect mode");
        }
        if let Some(span) = &effect.function_span {
            self.validate_span(source_file, span, "effect function");
        }
        if let Some(span) = &effect.call_span {
            self.validate_span(source_file, span, "effect call");
        }
        for span in &effect.arg_spans {
            self.validate_span(source_file, span, "effect argument");
        }
        self.validate_arguments(&effect.args, effect.span.clone(), "effect argument");
        self.validate_effect_schema(source_file, effect);
    }
}
