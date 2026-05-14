use recite_core::{
    Choice, CompiledStatement, CompiledStatementKind, DivertTarget, Effect, IfBranch, Line,
    SourceMapIndex, Statement, StatementIndex, StatementRange,
};

use super::{AssetBuilder, ReservedStatement, StatementPlan};
use crate::compile::CompileError;
use crate::compile::convert::{compile_condition_call, compile_condition_expression};
use crate::compile::table::usize_to_u32;

impl AssetBuilder<'_> {
    pub(super) fn compile_statement_range(
        &mut self,
        statements: &[Statement],
    ) -> Result<StatementRange, CompileError> {
        let start = StatementIndex::new(usize_to_u32("statements", self.statements.len())?);
        let mut reserved = Vec::new();
        let mut index = 0;

        while index < statements.len() {
            match &statements[index] {
                Statement::Comment(_) => index += 1,
                Statement::Choice(_) => {
                    reserved.push(self.reserve_standalone_prompt(statements, &mut index)?);
                }
                statement => {
                    reserved.push(self.reserve_planned_statement(statement)?);
                    index += 1;
                }
            }
        }

        let emitted = reserved
            .len()
            .try_into()
            .map_err(|_| CompileError::TableIndexOverflow {
                table: "statements",
                len: reserved.len(),
            })?;
        for statement in reserved {
            self.fill_statement(statement)?;
        }

        Ok(StatementRange::new(start, emitted))
    }

    fn reserve_standalone_prompt<'b>(
        &mut self,
        statements: &'b [Statement],
        index: &mut usize,
    ) -> Result<ReservedStatement<'b>, CompileError> {
        let source_map = match &statements[*index] {
            Statement::Choice(choice) => self.push_source_map(&choice.span)?,
            _ => unreachable!("standalone prompt starts at a choice"),
        };
        let statement_index = self.reserve_statement(source_map);
        let mut choices = Vec::new();

        while *index < statements.len() {
            match &statements[*index] {
                Statement::Choice(choice) => {
                    choices.push(choice);
                    *index += 1;
                }
                Statement::Comment(_) => *index += 1,
                _ => break,
            }
        }

        Ok(ReservedStatement {
            index: statement_index,
            plan: StatementPlan::StandalonePrompt(choices),
        })
    }

    fn reserve_planned_statement<'b>(
        &mut self,
        statement: &'b Statement,
    ) -> Result<ReservedStatement<'b>, CompileError> {
        let (span, plan) = match statement {
            Statement::Line(line) => (&line.span, StatementPlan::Line(line)),
            Statement::Divert(divert) => (&divert.span, StatementPlan::Divert(&divert.target)),
            Statement::If(branch) => (&branch.span, StatementPlan::If(branch)),
            Statement::Match(branch) => (&branch.span, StatementPlan::Match(branch)),
            Statement::Effect(effect) => (&effect.span, StatementPlan::Effect(effect)),
            Statement::Comment(_) | Statement::Choice(_) => {
                unreachable!("comments and standalone choices are handled by caller")
            }
        };
        let source_map = self.push_source_map(span)?;

        Ok(ReservedStatement {
            index: self.reserve_statement(source_map),
            plan,
        })
    }

    fn fill_statement(&mut self, statement: ReservedStatement<'_>) -> Result<(), CompileError> {
        let kind = match statement.plan {
            StatementPlan::Line(line) => self.compile_line_statement_kind(line)?,
            StatementPlan::StandalonePrompt(choices) => {
                self.compile_standalone_prompt_kind(choices)?
            }
            StatementPlan::Divert(target) => self.compile_divert_statement_kind(target)?,
            StatementPlan::If(branch) => self.compile_if_statement_kind(branch)?,
            StatementPlan::Match(branch) => self.compile_match_statement_kind(branch)?,
            StatementPlan::Effect(effect) => self.compile_effect_statement_kind(effect)?,
        };
        self.statements[statement.index].kind = kind;

        Ok(())
    }

    fn compile_line_statement_kind(
        &mut self,
        line: &Line,
    ) -> Result<CompiledStatementKind, CompileError> {
        let line_index = self.compile_line_row(line)?;
        if line.statements.is_empty() {
            return Ok(CompiledStatementKind::Line(line_index));
        }

        let choices = self.compile_choices(line.statements.iter().map(|statement| {
            let Statement::Choice(choice) = statement else {
                unreachable!("line child statement validation runs before asset output")
            };
            choice
        }))?;
        Ok(CompiledStatementKind::Prompt {
            line: Some(line_index),
            choices,
        })
    }

    fn compile_standalone_prompt_kind(
        &mut self,
        choices: Vec<&Choice>,
    ) -> Result<CompiledStatementKind, CompileError> {
        let choices = self.compile_choices(choices)?;
        Ok(CompiledStatementKind::Prompt {
            line: None,
            choices,
        })
    }

    fn compile_divert_statement_kind(
        &mut self,
        target: &DivertTarget,
    ) -> Result<CompiledStatementKind, CompileError> {
        Ok(match target {
            DivertTarget::Block(_) => {
                CompiledStatementKind::Divert(self.compile_divert_target(target)?)
            }
            DivertTarget::End => CompiledStatementKind::End,
        })
    }

    fn compile_if_statement_kind(
        &mut self,
        branch: &IfBranch,
    ) -> Result<CompiledStatementKind, CompileError> {
        let then_statements = self.compile_statement_range(&branch.then_statements)?;
        let else_statements = self.compile_statement_range(&branch.else_statements)?;
        Ok(CompiledStatementKind::If {
            condition: compile_condition_expression(&branch.condition),
            then_statements,
            else_statements,
        })
    }

    fn compile_match_statement_kind(
        &mut self,
        branch: &recite_core::MatchBranch,
    ) -> Result<CompiledStatementKind, CompileError> {
        let arms = self.compile_match_arms(&branch.arms)?;
        Ok(CompiledStatementKind::Match {
            scrutinee: compile_condition_call(&branch.scrutinee),
            arms,
        })
    }

    fn compile_effect_statement_kind(
        &mut self,
        effect: &Effect,
    ) -> Result<CompiledStatementKind, CompileError> {
        let effect_index = self.compile_effect_row(effect)?;
        Ok(CompiledStatementKind::Effect(effect_index))
    }

    fn reserve_statement(&mut self, source_map: SourceMapIndex) -> usize {
        let index = self.statements.len();
        self.statements.push(CompiledStatement {
            kind: CompiledStatementKind::End,
            source_map,
        });
        index
    }
}
