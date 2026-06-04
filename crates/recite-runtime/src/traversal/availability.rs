use recite_core::{
    CompiledArgument, CompiledAvailabilityReasonArgValue, CompiledConditionCall,
    CompiledConditionExpression, ScalarValue,
};

use crate::DialogueError;
use crate::context::{ConditionExpectedType, ConditionQuery, ConditionValue, DialogueContext};
use crate::event::{
    ChoiceAvailability, ChoiceAvailabilityReason, ChoiceAvailabilityReasonArg,
    ChoiceAvailabilityReasonOrigin, ChoiceAvailabilityReasonTree,
};

use super::asset::AssetView;
use super::malformed;

const MAX_AVAILABILITY_CONDITION_DEPTH: usize = 128;

pub(super) fn choice_availability(
    asset: AssetView<'_>,
    requirement: Option<&CompiledConditionExpression>,
    requirement_source_text: Option<&str>,
    primary_reason_override: Option<&recite_core::AvailabilityReasonId>,
    context: &dyn DialogueContext,
) -> Result<ChoiceAvailability, DialogueError> {
    let Some(requirement) = requirement else {
        return Ok(ChoiceAvailability::available());
    };

    let (is_available, reason_tree) =
        evaluate_availability_expression(asset, context, requirement, requirement_source_text, 0)?;
    if is_available {
        return Ok(ChoiceAvailability::available());
    }

    Ok(ChoiceAvailability::unavailable(
        primary_reason_override.and_then(|reason| {
            reason_for_id(
                asset,
                reason,
                requirement_source_text.map(|source_text| {
                    ChoiceAvailabilityReasonOrigin::RequirementExpression {
                        source_text: source_text.to_owned(),
                    }
                }),
            )
        }),
        reason_tree,
    ))
}

fn evaluate_availability_expression(
    asset: AssetView<'_>,
    context: &dyn DialogueContext,
    expression: &CompiledConditionExpression,
    requirement_source_text: Option<&str>,
    depth: usize,
) -> Result<(bool, Option<ChoiceAvailabilityReasonTree>), DialogueError> {
    if depth > MAX_AVAILABILITY_CONDITION_DEPTH {
        return Err(DialogueError::ConditionDepthLimitExceeded {
            limit: MAX_AVAILABILITY_CONDITION_DEPTH,
        });
    }

    match expression {
        CompiledConditionExpression::Call(call) => {
            evaluate_availability_call(asset, context, call, requirement_source_text)
        }
        CompiledConditionExpression::And(expressions) => evaluate_availability_group(
            asset,
            context,
            expressions,
            requirement_source_text,
            depth,
            ChoiceAvailabilityReasonTree::All,
            true,
        ),
        CompiledConditionExpression::Or(expressions) => evaluate_availability_group(
            asset,
            context,
            expressions,
            requirement_source_text,
            depth,
            ChoiceAvailabilityReasonTree::Any,
            false,
        ),
        CompiledConditionExpression::Not(expression) => {
            let (child_available, _) = evaluate_availability_expression(
                asset,
                context,
                expression,
                requirement_source_text,
                depth + 1,
            )?;
            Ok((!child_available, None))
        }
    }
}

fn evaluate_availability_group(
    asset: AssetView<'_>,
    context: &dyn DialogueContext,
    expressions: &[CompiledConditionExpression],
    requirement_source_text: Option<&str>,
    depth: usize,
    group: impl FnOnce(Vec<ChoiceAvailabilityReasonTree>) -> ChoiceAvailabilityReasonTree,
    all_must_pass: bool,
) -> Result<(bool, Option<ChoiceAvailabilityReasonTree>), DialogueError> {
    if expressions.is_empty() {
        let operator = if all_must_pass { "and" } else { "or" };
        return Err(malformed(format!(
            "condition `{operator}` expression has no children"
        )));
    }

    let mut failed_children = Vec::new();
    let mut any_passed = false;
    let mut any_failed = false;
    for expression in expressions {
        let (is_available, child_tree) = evaluate_availability_expression(
            asset,
            context,
            expression,
            requirement_source_text,
            depth + 1,
        )?;
        any_passed |= is_available;
        if !is_available {
            any_failed = true;
            if let Some(child_tree) = child_tree {
                failed_children.push(child_tree);
            }
        }
    }

    let is_available = if all_must_pass {
        !any_failed
    } else {
        any_passed
    };
    let reason_tree =
        (!is_available && !failed_children.is_empty()).then(|| group(failed_children));
    Ok((is_available, reason_tree))
}

fn evaluate_availability_call(
    asset: AssetView<'_>,
    context: &dyn DialogueContext,
    call: &CompiledConditionCall,
    requirement_source_text: Option<&str>,
) -> Result<(bool, Option<ChoiceAvailabilityReasonTree>), DialogueError> {
    let is_available = match context
        .evaluate_condition(ConditionQuery::new(
            &call.function,
            &call.args,
            ConditionExpectedType::Bool,
        ))
        .map_err(|error| DialogueError::ConditionEvaluationFailed {
            function: call.function.clone(),
            reason: error.reason().to_owned(),
        })? {
        ConditionValue::Bool(value) => value,
        value => {
            return Err(DialogueError::ConditionResultTypeMismatch {
                function: call.function.clone(),
                expected: ConditionExpectedType::Bool,
                actual: value.kind(),
            });
        }
    };

    if is_available {
        return Ok((true, None));
    }

    Ok((
        false,
        reason_for_call(asset, call)
            .map(ChoiceAvailabilityReasonTree::Reason)
            .or_else(|| {
                requirement_source_text
                    .map(str::to_owned)
                    .map(ChoiceAvailabilityReasonTree::RequirementSourceText)
            }),
    ))
}

fn reason_for_call(
    asset: AssetView<'_>,
    call: &CompiledConditionCall,
) -> Option<ChoiceAvailabilityReason> {
    let mapping = asset.condition_availability_reason(&call.function)?;
    reason_for_id_with_args(
        asset,
        &mapping.reason,
        Some(ChoiceAvailabilityReasonOrigin::ConditionCall {
            function: call.function.clone(),
            args: call.args.iter().map(argument_value).collect(),
        }),
        mapping.args.iter().filter_map(|binding| {
            reason_arg_value(&binding.value, call).map(|value| ChoiceAvailabilityReasonArg {
                name: binding.name.clone(),
                value,
            })
        }),
    )
}

fn reason_for_id(
    asset: AssetView<'_>,
    reason_id: &recite_core::AvailabilityReasonId,
    origin: Option<ChoiceAvailabilityReasonOrigin>,
) -> Option<ChoiceAvailabilityReason> {
    reason_for_id_with_args(asset, reason_id, origin, std::iter::empty())
}

fn reason_for_id_with_args(
    asset: AssetView<'_>,
    reason_id: &recite_core::AvailabilityReasonId,
    origin: Option<ChoiceAvailabilityReasonOrigin>,
    args: impl IntoIterator<Item = ChoiceAvailabilityReasonArg>,
) -> Option<ChoiceAvailabilityReason> {
    let reason = asset.availability_reason(reason_id)?;
    Some(ChoiceAvailabilityReason {
        id: reason.id.clone(),
        source_text: reason.template.clone(),
        origin,
        args: args.into_iter().collect(),
    })
}

fn reason_arg_value(
    value: &CompiledAvailabilityReasonArgValue,
    call: &CompiledConditionCall,
) -> Option<String> {
    match value {
        CompiledAvailabilityReasonArgValue::ConditionArg(name) => {
            condition_argument_value(name, &call.args)
        }
        CompiledAvailabilityReasonArgValue::Literal(value) => Some(value.clone()),
    }
}

fn condition_argument_value(name: &str, args: &[CompiledArgument]) -> Option<String> {
    let index = name.parse::<usize>().ok()?;
    args.get(index).map(argument_value)
}

fn argument_value(argument: &CompiledArgument) -> String {
    match argument {
        CompiledArgument::Identifier(value) => value.clone(),
        CompiledArgument::Value(ScalarValue::String(value)) => value.clone(),
        CompiledArgument::Value(ScalarValue::Integer(value)) => value.to_string(),
        CompiledArgument::Value(ScalarValue::Float(value)) => value.to_string(),
        CompiledArgument::Value(ScalarValue::Boolean(value)) => value.to_string(),
    }
}
