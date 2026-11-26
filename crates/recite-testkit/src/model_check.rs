//! Check that lowering a rendered program keeps the generator's structure.
//!
//! The reference interpreter and the compiler both read the parser's AST, so
//! a lowering mistake, such as attaching a standalone choice to the line
//! before it or splitting a prompt at a comment, would make them agree on
//! the wrong program. This compares the AST against the model it came from.

use recite_core::ast::{
    Argument, ConditionCall, ConditionExpression, DivertTarget, MatchPattern, Statement,
};

use crate::oracle::{ReferenceProgram, effect_observation};
use crate::program::{ChoiceSpec, Cond, Program, Stmt, Target};

impl ReferenceProgram {
    /// Whether the lowered AST has the model's blocks, statements, nesting,
    /// IDs, effects, branches and targets.
    pub fn check_model(&self, program: &Program) -> Result<(), String> {
        if self.files.len() != program.files.len() {
            return Err(format!(
                "{} files lowered, {} rendered",
                self.files.len(),
                program.files.len()
            ));
        }
        for (file, spec) in self.files.iter().zip(&program.files) {
            let ids: Vec<_> = file.blocks.iter().map(|block| block.id.as_str()).collect();
            let expected: Vec<_> = spec.blocks.iter().map(|block| block.id.as_str()).collect();
            if ids != expected {
                return Err(format!(
                    "{}: blocks {ids:?}, expected {expected:?}",
                    spec.path
                ));
            }
            for (block, block_spec) in file.blocks.iter().zip(&spec.blocks) {
                let checker = Checker {
                    program,
                    file: &spec.path,
                };
                checker
                    .list(&block.statements, &block_spec.body)
                    .map_err(|error| format!("{}::{}: {error}", spec.path, block_spec.id))?;
            }
        }
        Ok(())
    }
}

struct Checker<'a> {
    program: &'a Program,
    file: &'a str,
}

impl Checker<'_> {
    fn list(&self, lowered: &[Statement], model: &[Stmt]) -> Result<(), String> {
        let mut index = 0;
        for statement in model {
            match statement {
                Stmt::Choices(choices) => {
                    // A standalone prompt is a run of choices with comments between.
                    let mut found = Vec::new();
                    while let Some(next) = lowered.get(index) {
                        match next {
                            Statement::Choice(choice) => found.push(choice),
                            Statement::Comment(_) if found.len() < choices.len() => {}
                            _ => break,
                        }
                        index += 1;
                        if found.len() == choices.len() {
                            break;
                        }
                    }
                    if found.len() != choices.len() {
                        return Err(format!(
                            "prompt has {} choices, expected {}",
                            found.len(),
                            choices.len()
                        ));
                    }
                    for (choice, spec) in found.into_iter().zip(choices) {
                        self.choice(choice, spec)?;
                    }
                }
                other => {
                    let lowered = lowered
                        .get(index)
                        .ok_or_else(|| format!("missing statement for {other:?}"))?;
                    index += 1;
                    self.statement(lowered, other)?;
                }
            }
        }
        match lowered.get(index) {
            Some(extra) => Err(format!("unexpected statement {extra:?}")),
            None => Ok(()),
        }
    }

    fn statement(&self, lowered: &Statement, model: &Stmt) -> Result<(), String> {
        match (lowered, model) {
            (Statement::Comment(_), Stmt::Comment) => Ok(()),
            (Statement::Line(line), Stmt::Line(spec)) => {
                same(
                    "line id",
                    line.id.as_ref().map(|id| id.as_str()),
                    Some(spec.id.as_str()),
                )?;
                let choices: Vec<_> = line
                    .statements
                    .iter()
                    .map(|statement| match statement {
                        Statement::Choice(choice) => Ok(choice),
                        other => Err(format!("line {} holds {other:?}", spec.id)),
                    })
                    .collect::<Result<_, _>>()?;
                same("line choices", choices.len(), spec.choices.len())?;
                for (choice, choice_spec) in choices.into_iter().zip(&spec.choices) {
                    self.choice(choice, choice_spec)?;
                }
                Ok(())
            }
            (Statement::Effect(effect), Stmt::Effect(spec)) => {
                let effect = effect_observation(effect);
                same("effect mode", effect.mode, spec.mode())?;
                same("effect function", effect.function.as_str(), spec.function())?;
                let args: Vec<_> = spec
                    .actor()
                    .map(|actor| actor.name().to_owned())
                    .into_iter()
                    .collect();
                same("effect arguments", effect.args, args)
            }
            (
                Statement::If(branch),
                Stmt::If {
                    condition,
                    then,
                    otherwise,
                },
            ) => {
                same(
                    "condition",
                    lowered_condition(&branch.condition),
                    model_condition(condition),
                )?;
                self.list(&branch.then_statements, then)?;
                self.list(&branch.else_statements, otherwise)
            }
            (Statement::Match(branch), Stmt::Match { scrutinee, arms }) => {
                same(
                    "scrutinee",
                    lowered_call(&branch.scrutinee),
                    crate::render::scrutinee_call(*scrutinee),
                )?;
                same("arms", branch.arms.len(), arms.len())?;
                for (arm, spec) in branch.arms.iter().zip(arms) {
                    let pattern = match &arm.pattern {
                        MatchPattern::Variant(variant) => Some(variant.as_str()),
                        MatchPattern::Wildcard => None,
                    };
                    same("arm pattern", pattern, spec.pattern)?;
                    self.list(&arm.statements, &spec.body)?;
                }
                Ok(())
            }
            (Statement::Divert(divert), Stmt::Divert(target)) => {
                self.target(&divert.target, *target)
            }
            (lowered, model) => Err(format!("lowered {lowered:?} for {model:?}")),
        }
    }

    fn choice(&self, choice: &recite_core::ast::Choice, spec: &ChoiceSpec) -> Result<(), String> {
        same(
            "choice id",
            choice.id.as_ref().map(|id| id.as_str()),
            Some(spec.id.as_str()),
        )?;
        same(
            "choice requirement",
            choice
                .availability_requirement
                .as_ref()
                .map(|requirement| lowered_condition(&requirement.condition)),
            spec.requires.as_ref().map(model_condition),
        )?;
        let target = choice
            .target
            .as_ref()
            .ok_or_else(|| format!("choice {} has no target", spec.id))?;
        self.target(&target.target, spec.target)
    }

    fn target(&self, lowered: &DivertTarget, model: Target) -> Result<(), String> {
        let expected = match model {
            Target::End => None,
            Target::Block { file, block } => {
                let spec = &self.program.files[file];
                Some((
                    (spec.path != self.file).then_some(spec.path.as_str()),
                    spec.blocks[block].id.as_str(),
                ))
            }
        };
        let actual = match lowered {
            DivertTarget::End => None,
            DivertTarget::Block(reference) => {
                Some((reference.file.as_deref(), reference.block_id.as_str()))
            }
        };
        same("target", actual, expected)
    }
}

/// A lowered condition's structure: calls with their arguments, `and`, `or`
/// and `not`, without spans or grouping parentheses.
fn lowered_condition(condition: &ConditionExpression) -> String {
    let group = |operator: &str, operands: &[ConditionExpression]| {
        let operands: Vec<_> = operands.iter().map(lowered_condition).collect();
        format!("{operator}({})", operands.join(", "))
    };
    match condition {
        ConditionExpression::Call(call) => lowered_call(call),
        ConditionExpression::And(operands) => group("and", &operands.expressions),
        ConditionExpression::Or(operands) => group("or", &operands.expressions),
        ConditionExpression::Not(unary) => format!("not({})", lowered_condition(&unary.expression)),
        ConditionExpression::Grouped(unary) => lowered_condition(&unary.expression),
    }
}

fn lowered_call(call: &ConditionCall) -> String {
    let args: Vec<_> = call
        .args
        .iter()
        .map(|argument| match argument {
            Argument::Identifier(value) => value.clone(),
            Argument::Value(value) => format!("{value:?}"),
        })
        .collect();
    format!("{}({})", call.function, args.join(", "))
}

/// The model condition in the form of [`lowered_condition`].
fn model_condition(condition: &Cond) -> String {
    let group = |operator: &str, operands: &[Cond]| {
        let operands: Vec<_> = operands.iter().map(model_condition).collect();
        format!("{operator}({})", operands.join(", "))
    };
    match condition {
        Cond::Flag(actor) => format!("flag({})", actor.name()),
        Cond::Ready => "ready()".to_owned(),
        Cond::Not(operand) => format!("not({})", model_condition(operand)),
        Cond::And(operands) => group("and", operands),
        Cond::Or(operands) => group("or", operands),
    }
}

fn same<T: PartialEq + std::fmt::Debug>(what: &str, actual: T, expected: T) -> Result<(), String> {
    if actual == expected {
        Ok(())
    } else {
        Err(format!("{what} is {actual:?}, expected {expected:?}"))
    }
}
