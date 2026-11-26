//! The host-observable save points of a program, from its source AST.
//!
//! A point is reachable when some host answers reach it. Branches are
//! explored independently, as a host may answer each condition call either
//! way; a `:case _` counts even after arms for every declared variant,
//! because a compiled asset does not carry the enum and a later schema may
//! add a variant without recompiling dialogue.

use recite_core::ast::Statement;

use crate::oracle::{ReferenceProgram, effect_observation};
use crate::program::EffectMode;

/// What a save point is taken after, mirroring the runtime's save-point
/// kinds.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum SavePointKey {
    BlockStart,
    AfterLine(String),
    Prompt {
        line: Option<String>,
        choices: Vec<String>,
    },
    BlockingEffect(String),
    AfterImmediateEffect(String),
}

impl ReferenceProgram {
    /// Every reachable save point as `(block ID, key)`, sorted.
    #[must_use]
    pub fn save_points(&self) -> Vec<(String, SavePointKey)> {
        let mut points = Vec::new();
        for file in &self.files {
            for block in &file.blocks {
                let id = block.id.as_str().to_owned();
                points.push((id.clone(), SavePointKey::BlockStart));
                walk(&block.statements, &mut |key| points.push((id.clone(), key)));
            }
        }
        points.sort();
        points
    }
}

/// Walks a statement list and returns whether traversal always leaves it
/// before its end, which makes the statements after its branch unreachable.
fn walk(statements: &[Statement], point: &mut dyn FnMut(SavePointKey)) -> bool {
    let mut index = 0;
    while let Some(statement) = statements.get(index) {
        index += 1;
        match statement {
            Statement::Comment(_) => {}
            Statement::Line(line) => {
                let id = line
                    .id
                    .as_ref()
                    .map(|id| id.as_str().to_owned())
                    .unwrap_or_default();
                if line.statements.is_empty() {
                    point(SavePointKey::AfterLine(id));
                } else {
                    point(SavePointKey::Prompt {
                        line: Some(id),
                        choices: choice_ids(&line.statements),
                    });
                    return true;
                }
            }
            Statement::Choice(_) => {
                let group = statements[index - 1..]
                    .iter()
                    .take_while(|statement| {
                        matches!(statement, Statement::Choice(_) | Statement::Comment(_))
                    })
                    .cloned()
                    .collect::<Vec<_>>();
                point(SavePointKey::Prompt {
                    line: None,
                    choices: choice_ids(&group),
                });
                return true;
            }
            Statement::Effect(effect) => {
                let effect = effect_observation(effect);
                match effect.mode {
                    EffectMode::Immediate => {
                        point(SavePointKey::AfterImmediateEffect(effect.function));
                    }
                    EffectMode::Blocking => point(SavePointKey::BlockingEffect(effect.function)),
                    EffectMode::Deferred => {}
                }
            }
            Statement::If(branch) => {
                let then_leaves = walk(&branch.then_statements, point);
                let else_leaves = walk(&branch.else_statements, point);
                if then_leaves && else_leaves {
                    return true;
                }
            }
            Statement::Match(branch) => {
                // Only the first arm for a variant, and the first wildcard,
                // can ever be selected.
                let mut seen = Vec::new();
                let mut every_arm_leaves = true;
                for arm in &branch.arms {
                    if seen.contains(&&arm.pattern) {
                        continue;
                    }
                    seen.push(&arm.pattern);
                    every_arm_leaves &= walk(&arm.statements, point);
                }
                // Without a matching arm traversal stops with an error, so a
                // match whose every arm leaves is never passed either.
                if every_arm_leaves {
                    return true;
                }
            }
            Statement::Divert(_) => return true,
        }
    }
    false
}

fn choice_ids(statements: &[Statement]) -> Vec<String> {
    statements
        .iter()
        .filter_map(|statement| match statement {
            Statement::Choice(choice) => choice.id.as_ref().map(|id| id.as_str().to_owned()),
            _ => None,
        })
        .collect()
}
