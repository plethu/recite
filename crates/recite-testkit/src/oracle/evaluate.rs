//! Condition evaluation and effect projection for the reference interpreter.

use recite_core::ast::{self, Argument, ConditionExpression, Effect};

use super::OracleSession;
use crate::answers::Answer;
use crate::observation::{EffectObservation, Rejection};
use crate::program::EffectMode;

impl OracleSession<'_> {
    pub(super) fn condition(&self, expression: &ConditionExpression) -> Result<bool, Rejection> {
        match expression {
            ConditionExpression::Call(call) => match self.call(call)? {
                Answer::Bool(value) => Ok(value),
                Answer::Variant(_) => Err(Rejection::Other("condition returned a variant".into())),
            },
            ConditionExpression::And(group) => {
                let mut all = true;
                for operand in &group.expressions {
                    all &= self.condition(operand)?;
                }
                Ok(all)
            }
            ConditionExpression::Or(group) => {
                let mut any = false;
                for operand in &group.expressions {
                    any |= self.condition(operand)?;
                }
                Ok(any)
            }
            ConditionExpression::Not(unary) => Ok(!self.condition(&unary.expression)?),
            ConditionExpression::Grouped(unary) => self.condition(&unary.expression),
        }
    }

    pub(super) fn call(&self, call: &ast::ConditionCall) -> Result<Answer, Rejection> {
        let args: Vec<String> = call.args.iter().map(argument).collect();
        let args: Vec<&str> = args.iter().map(String::as_str).collect();
        self.answers
            .answer(&call.function, &args)
            .ok_or_else(|| Rejection::Other(format!("no answer for {}", call.function)))
    }
}

pub(crate) fn effect_observation(effect: &Effect) -> EffectObservation {
    EffectObservation {
        mode: match effect.mode {
            ast::EffectMode::Immediate => EffectMode::Immediate,
            ast::EffectMode::Deferred => EffectMode::Deferred,
            ast::EffectMode::Blocking => EffectMode::Blocking,
        },
        function: effect.function.clone(),
        args: effect.args.iter().map(argument).collect(),
    }
}

fn argument(argument: &Argument) -> String {
    match argument {
        Argument::Identifier(value) => value.clone(),
        Argument::Value(value) => format!("{value:?}"),
    }
}
