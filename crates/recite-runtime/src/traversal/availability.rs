use recite_core::{
    CompiledArgument, CompiledAvailabilityReasonArgValue, CompiledConditionCall,
    CompiledConditionExpression, ScalarValue,
};

use crate::DialogueError;
use crate::context::DialogueContext;
use crate::event::{
    ChoiceAvailability, ChoiceAvailabilityReason, ChoiceAvailabilityReasonArg,
    ChoiceAvailabilityReasonTree,
};

use super::asset::AssetView;
use super::condition::evaluate_condition;

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

    if evaluate_condition(context, requirement)? {
        return Ok(ChoiceAvailability::available());
    }

    Ok(ChoiceAvailability::unavailable(
        primary_reason_override.and_then(|reason| reason_for_id(asset, reason)),
        reason_tree_for_expression(asset, requirement, requirement_source_text),
    ))
}

fn reason_tree_for_expression(
    asset: AssetView<'_>,
    expression: &CompiledConditionExpression,
    requirement_source_text: Option<&str>,
) -> Option<ChoiceAvailabilityReasonTree> {
    match expression {
        CompiledConditionExpression::Call(call) => reason_for_call(asset, call)
            .map(ChoiceAvailabilityReasonTree::Reason)
            .or_else(|| {
                requirement_source_text
                    .map(str::to_owned)
                    .map(ChoiceAvailabilityReasonTree::RequirementSourceText)
            }),
        CompiledConditionExpression::And(expressions) => grouped_reason_tree(
            asset,
            expressions,
            requirement_source_text,
            ChoiceAvailabilityReasonTree::All,
        ),
        CompiledConditionExpression::Or(expressions) => grouped_reason_tree(
            asset,
            expressions,
            requirement_source_text,
            ChoiceAvailabilityReasonTree::Any,
        ),
        CompiledConditionExpression::Not(expression) => {
            reason_tree_for_expression(asset, expression, requirement_source_text)
                .map(Box::new)
                .map(ChoiceAvailabilityReasonTree::Not)
        }
    }
}

fn grouped_reason_tree(
    asset: AssetView<'_>,
    expressions: &[CompiledConditionExpression],
    requirement_source_text: Option<&str>,
    group: impl FnOnce(Vec<ChoiceAvailabilityReasonTree>) -> ChoiceAvailabilityReasonTree,
) -> Option<ChoiceAvailabilityReasonTree> {
    let children = expressions
        .iter()
        .filter_map(|expression| {
            reason_tree_for_expression(asset, expression, requirement_source_text)
        })
        .collect::<Vec<_>>();

    (!children.is_empty()).then(|| group(children))
}

fn reason_for_call(
    asset: AssetView<'_>,
    call: &CompiledConditionCall,
) -> Option<ChoiceAvailabilityReason> {
    let mapping = asset.condition_availability_reason(&call.function)?;
    reason_for_id_with_args(
        asset,
        &mapping.reason,
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
) -> Option<ChoiceAvailabilityReason> {
    reason_for_id_with_args(asset, reason_id, std::iter::empty())
}

fn reason_for_id_with_args(
    asset: AssetView<'_>,
    reason_id: &recite_core::AvailabilityReasonId,
    args: impl IntoIterator<Item = ChoiceAvailabilityReasonArg>,
) -> Option<ChoiceAvailabilityReason> {
    let reason = asset.availability_reason(reason_id)?;
    Some(ChoiceAvailabilityReason {
        id: reason.id.clone(),
        source_text: reason.template.clone(),
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
