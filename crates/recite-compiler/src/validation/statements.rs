use recite_core::{
    Block, Choice, Divert, Effect, IfBranch, Line, MatchArm, MatchBranch, MetadataTarget,
    SourceFile, SourceId, Statement,
};

use super::metadata::MetadataValidationContext;
use super::participation::ValidationCompleteness;
use super::state::Validator;
use crate::diagnostics::{self, ArgumentOwner as A, SourceSpanOwner as O};

mod interpolation;

impl<'a> Validator<'a> {
    pub(super) fn validate_block(&mut self, source_file: &'a SourceFile, block: &'a Block) {
        if self.participation.ast_structure() == ValidationCompleteness::Complete {
            self.validate_span(source_file, &block.span, O::Block);
        }
        if self.participation.ast_structure() == ValidationCompleteness::Complete
            && self.participation.metadata() == ValidationCompleteness::Complete
        {
            self.validate_metadata(
                source_file,
                MetadataValidationContext {
                    target: MetadataTarget::Block,
                    line_speaker: None,
                    block_default_speaker: None,
                    metadata: &block.metadata,
                },
            );
        }
        if self.participation.block_definitions() == ValidationCompleteness::Complete {
            self.validate_block_id(source_file, block);
            self.validate_default_block(block);
        }
    }
    pub(super) fn validate_statement_with_block(
        &mut self,
        source_file: &'a SourceFile,
        statement: &'a Statement,
        block_default_speaker: Option<&'a str>,
    ) {
        match statement {
            Statement::Line(line) => {
                self.validate_line(source_file, line, block_default_speaker);
                for statement in &line.statements {
                    if self.participation.ast_structure() == ValidationCompleteness::Complete
                        && !matches!(statement, Statement::Choice(_))
                    {
                        self.diagnostics
                            .push(diagnostics::unsupported_line_child_statement(
                                line, statement,
                            ));
                    }
                    self.validate_statement_with_block(
                        source_file,
                        statement,
                        block_default_speaker,
                    );
                }
            }
            Statement::Choice(choice) => {
                self.validate_choice(source_file, choice);
                for statement in &choice.statements {
                    if self.participation.ast_structure() == ValidationCompleteness::Complete {
                        self.diagnostics
                            .push(diagnostics::unsupported_choice_child_statement(
                                choice, statement,
                            ));
                    }
                    self.validate_statement_with_block(
                        source_file,
                        statement,
                        block_default_speaker,
                    );
                }
            }
            Statement::Divert(divert) => self.validate_divert(source_file, divert),
            Statement::If(branch) => {
                self.validate_if_branch(source_file, branch);
                for statement in &branch.then_statements {
                    self.validate_statement_with_block(
                        source_file,
                        statement,
                        block_default_speaker,
                    );
                }
                for statement in &branch.else_statements {
                    self.validate_statement_with_block(
                        source_file,
                        statement,
                        block_default_speaker,
                    );
                }
            }
            Statement::Match(branch) => {
                self.validate_match_branch(source_file, branch);
                for arm in &branch.arms {
                    self.validate_match_arm(source_file, arm);
                    for statement in &arm.statements {
                        self.validate_statement_with_block(
                            source_file,
                            statement,
                            block_default_speaker,
                        );
                    }
                }
            }
            Statement::Effect(effect) => self.validate_effect(source_file, effect),
            Statement::Comment(comment) => {
                if self.participation.ast_structure() == ValidationCompleteness::Complete {
                    self.validate_span(source_file, &comment.span, O::Comment);
                }
            }
        }
    }
    pub(super) fn validate_line(
        &mut self,
        source_file: &'a SourceFile,
        line: &'a Line,
        block_default_speaker: Option<&'a str>,
    ) {
        if self.participation.ast_structure() == ValidationCompleteness::Complete {
            self.validate_span(source_file, &line.span, O::Line);
            self.validate_source_text(
                source_file,
                &line.source_text,
                O::LineSourceText,
                self.participation,
            );
            if let Some(plural_source_text) = &line.plural_source_text {
                self.validate_plural_line(source_file, line, plural_source_text);
            } else {
                self.validate_interpolation(&line.source_text, &line.interpolation_bindings);
            }
        }
        if self.participation.ast_structure() == ValidationCompleteness::Complete
            && self.participation.metadata() == ValidationCompleteness::Complete
        {
            self.validate_metadata(
                source_file,
                MetadataValidationContext {
                    target: MetadataTarget::Line,
                    line_speaker: line.speaker.as_ref().map(|speaker| speaker.as_str()),
                    block_default_speaker,
                    metadata: &line.metadata,
                },
            );
        }

        if self.participation.stable_ids() == ValidationCompleteness::Complete {
            self.validate_line_localisable_id(line);
        }
    }

    pub(crate) fn validate_line_localisable_id(&mut self, line: &'a Line) {
        let SourceId::Frozen { .. } = &line.source_id else {
            self.diagnostics.push(match &line.source_id {
                SourceId::Missing => diagnostics::missing_line_id(line),
                SourceId::Draft { .. } => diagnostics::draft_line_id(line),
                SourceId::Malformed { .. } => diagnostics::malformed_line_id(line, &line.source_id),
                SourceId::Frozen { .. } => unreachable!("frozen ID matched earlier"),
            });
            return;
        };
        let Some(id) = line.id.as_ref() else {
            return;
        };

        if let Some(first_span) = self.localisable_ids.get(id.as_str())
            && (self.project_complete || first_span.file == line.span.file)
        {
            self.diagnostics
                .push(diagnostics::duplicate_line_id(line, id, first_span.clone()));
        } else {
            self.localisable_ids.insert(id.as_str(), line.span.clone());
        }
    }
    pub(super) fn validate_choice(&mut self, source_file: &'a SourceFile, choice: &'a Choice) {
        if self.participation.ast_structure() == ValidationCompleteness::Complete {
            self.validate_span(source_file, &choice.span, O::Choice);
            self.validate_source_text(
                source_file,
                &choice.source_text,
                O::ChoiceSourceText,
                self.participation,
            );
            self.validate_interpolation(&choice.source_text, &choice.interpolation_bindings);
        }
        if self.participation.ast_structure() == ValidationCompleteness::Complete
            && self.participation.metadata() == ValidationCompleteness::Complete
        {
            self.validate_metadata(
                source_file,
                MetadataValidationContext {
                    target: MetadataTarget::Choice,
                    line_speaker: None,
                    block_default_speaker: None,
                    metadata: &choice.metadata,
                },
            );
        }
        if self.participation.stable_ids() == ValidationCompleteness::Complete {
            self.validate_choice_echo(choice);
        }
        if self.participation.ast_structure() == ValidationCompleteness::Complete
            && self.participation.condition_functions() == ValidationCompleteness::Complete
        {
            if let Some(requirement) = &choice.availability_requirement {
                self.validate_span(
                    source_file,
                    &requirement.span,
                    O::ChoiceAvailabilityRequirement,
                );
                self.validate_condition_expression(source_file, &requirement.condition);
                self.validate_boolean_condition_schema(&requirement.condition);
            }
            if let Some(reason) = &choice.availability_reason_override {
                self.validate_span(source_file, &reason.span, O::ChoiceAvailabilityReason);
                self.validate_span(source_file, &reason.id_span, O::ChoiceAvailabilityReasonId);
                if let Some(span) = &reason.argument_span {
                    self.validate_span(source_file, span, O::ChoiceAvailabilityReasonArguments);
                }
            }
            self.validate_choice_availability_reason(choice);
        }

        if self.participation.stable_ids() == ValidationCompleteness::Complete {
            self.validate_choice_localisable_id(choice);
        }

        if let Some(target) = &choice.target {
            if self.participation.ast_structure() == ValidationCompleteness::Complete {
                self.validate_span(source_file, &target.span, O::ChoiceTarget);
            }
            if self.participation.block_references() == ValidationCompleteness::Complete {
                self.validate_reference(source_file, &target.target, &target.span);
            }
        } else if self.participation.ast_structure() == ValidationCompleteness::Complete {
            self.diagnostics
                .push(diagnostics::missing_choice_target(choice));
        }
    }
    pub(crate) fn validate_choice_localisable_id(&mut self, choice: &'a Choice) {
        if let SourceId::Frozen { .. } = &choice.source_id {
            let Some(id) = choice.id.as_ref() else {
                return;
            };
            if let Some(first_span) = self.localisable_ids.get(id.as_str()) {
                self.diagnostics.push(diagnostics::duplicate_choice_id(
                    choice,
                    id,
                    first_span.clone(),
                ));
            } else {
                self.localisable_ids
                    .insert(id.as_str(), choice.span.clone());
            }
        } else {
            self.diagnostics.push(match &choice.source_id {
                SourceId::Missing => diagnostics::missing_choice_id(choice),
                SourceId::Draft { .. } => diagnostics::draft_choice_id(choice),
                SourceId::Malformed { .. } => {
                    diagnostics::malformed_choice_id(choice, &choice.source_id)
                }
                SourceId::Frozen { .. } => unreachable!("frozen ID matched earlier"),
            });
        }
    }
    pub(super) fn validate_divert(&mut self, source_file: &'a SourceFile, divert: &'a Divert) {
        if self.participation.ast_structure() == ValidationCompleteness::Complete {
            self.validate_span(source_file, &divert.span, O::Divert);
        }
        if self.participation.block_references() == ValidationCompleteness::Complete {
            self.validate_reference(source_file, &divert.target, &divert.span);
        }
    }
    pub(super) fn validate_if_branch(&mut self, source_file: &'a SourceFile, branch: &'a IfBranch) {
        if self.participation.ast_structure() == ValidationCompleteness::Complete {
            self.validate_span(source_file, &branch.span, O::IfBranch);
        }
        if self.participation.ast_structure() == ValidationCompleteness::Complete
            && self.participation.condition_functions() == ValidationCompleteness::Complete
        {
            self.validate_condition_expression(source_file, &branch.condition);
            self.validate_boolean_condition_schema(&branch.condition);
        }
    }
    pub(super) fn validate_match_branch(
        &mut self,
        source_file: &'a SourceFile,
        branch: &'a MatchBranch,
    ) {
        if self.participation.ast_structure() == ValidationCompleteness::Complete {
            self.validate_span(source_file, &branch.span, O::MatchBranch);
        }
        if self.participation.ast_structure() == ValidationCompleteness::Complete
            && self.participation.condition_functions() == ValidationCompleteness::Complete
        {
            self.validate_condition_call(source_file, &branch.scrutinee);
            self.validate_match_scrutinee_schema(&branch.scrutinee);
        }
    }
    pub(super) fn validate_match_arm(&mut self, source_file: &'a SourceFile, arm: &'a MatchArm) {
        if self.participation.ast_structure() == ValidationCompleteness::Complete {
            self.validate_span(source_file, &arm.span, O::MatchArm);
        }
    }
    pub(super) fn validate_effect(&mut self, source_file: &'a SourceFile, effect: &'a Effect) {
        if self.participation.ast_structure() == ValidationCompleteness::Complete {
            self.validate_span(source_file, &effect.span, O::Effect);
            if let Some(span) = &effect.mode_span {
                self.validate_span(source_file, span, O::EffectMode);
            }
            if let Some(span) = &effect.function_span {
                self.validate_span(source_file, span, O::EffectFunction);
            }
            if let Some(span) = &effect.call_span {
                self.validate_span(source_file, span, O::EffectCall);
            }
            for span in &effect.arg_spans {
                self.validate_span(source_file, span, O::EffectArgument);
            }
        }
        if self.participation.ast_structure() == ValidationCompleteness::Complete
            && self.participation.effect_functions() == ValidationCompleteness::Complete
        {
            self.validate_arguments(&effect.args, effect.span.clone(), A::Effect);
            self.validate_effect_schema(source_file, effect);
        }
    }
}
