use recite_core::{CompiledConditionCall, CompiledConditionExpression};

use crate::DialogueError;
use crate::context::{ConditionExpectedType, ConditionQuery, ConditionValue, DialogueContext};

use super::malformed;

const MAX_CONDITION_DEPTH: usize = 128;

pub(super) fn evaluate_condition(
    context: &dyn DialogueContext,
    condition: &CompiledConditionExpression,
) -> Result<bool, DialogueError> {
    evaluate_condition_at_depth(context, condition, 0)
}

fn evaluate_condition_at_depth(
    context: &dyn DialogueContext,
    condition: &CompiledConditionExpression,
    depth: usize,
) -> Result<bool, DialogueError> {
    if depth > MAX_CONDITION_DEPTH {
        return Err(DialogueError::ConditionDepthLimitExceeded {
            limit: MAX_CONDITION_DEPTH,
        });
    }

    match condition {
        CompiledConditionExpression::Call(call) => evaluate_condition_call(context, call),
        CompiledConditionExpression::And(expressions) => {
            if expressions.is_empty() {
                return Err(malformed(
                    "condition `and` expression has no children".to_owned(),
                ));
            }

            for expression in expressions {
                if !evaluate_condition_at_depth(context, expression, depth + 1)? {
                    return Ok(false);
                }
            }

            Ok(true)
        }
        CompiledConditionExpression::Or(expressions) => {
            if expressions.is_empty() {
                return Err(malformed(
                    "condition `or` expression has no children".to_owned(),
                ));
            }

            for expression in expressions {
                if evaluate_condition_at_depth(context, expression, depth + 1)? {
                    return Ok(true);
                }
            }

            Ok(false)
        }
        CompiledConditionExpression::Not(expression) => Ok(!evaluate_condition_at_depth(
            context,
            expression,
            depth + 1,
        )?),
    }
}

fn evaluate_condition_call(
    context: &dyn DialogueContext,
    call: &CompiledConditionCall,
) -> Result<bool, DialogueError> {
    match evaluate_condition_value(context, call, ConditionExpectedType::Bool)? {
        ConditionValue::Bool(value) => Ok(value),
        value => Err(DialogueError::ConditionResultTypeMismatch {
            function: call.function.clone(),
            expected: ConditionExpectedType::Bool,
            actual: value.kind(),
        }),
    }
}

pub(super) fn evaluate_enum_condition(
    context: &dyn DialogueContext,
    call: &CompiledConditionCall,
) -> Result<String, DialogueError> {
    match evaluate_condition_value(context, call, ConditionExpectedType::Enum)? {
        ConditionValue::EnumVariant(value) => Ok(value),
        value => Err(DialogueError::ConditionResultTypeMismatch {
            function: call.function.clone(),
            expected: ConditionExpectedType::Enum,
            actual: value.kind(),
        }),
    }
}

fn evaluate_condition_value(
    context: &dyn DialogueContext,
    call: &CompiledConditionCall,
    expected_type: ConditionExpectedType,
) -> Result<ConditionValue, DialogueError> {
    context
        .evaluate_condition(ConditionQuery::new(
            &call.function,
            &call.args,
            expected_type,
        ))
        .map_err(|error| DialogueError::ConditionEvaluationFailed {
            function: call.function.clone(),
            reason: error.reason().to_owned(),
        })
}
