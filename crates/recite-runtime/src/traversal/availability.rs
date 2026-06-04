use recite_core::{
    CompiledArgument, CompiledAvailabilityReasonArgValue, CompiledConditionCall,
    CompiledConditionExpression, ScalarValue,
};

use crate::DialogueError;
use crate::context::{ConditionExpectedType, ConditionQuery, ConditionValue, DialogueContext};
use crate::event::{
    ChoiceAvailability, ChoiceAvailabilityReason, ChoiceAvailabilityReasonArg,
    ChoiceAvailabilityReasonOrigin, ChoiceAvailabilityReasonTree, ChoiceAvailabilityReasonValue,
};
use crate::locale::TextDomain;

use super::asset::AssetView;
use super::malformed;
use super::output::LocaleLookup;

const MAX_AVAILABILITY_CONDITION_DEPTH: usize = 128;

#[derive(Clone, Copy)]
enum AvailabilityGroup {
    All,
    Any,
}

pub(super) fn choice_availability(
    asset: AssetView<'_>,
    requirement: Option<&CompiledConditionExpression>,
    requirement_source_text: Option<&str>,
    primary_reason_override: Option<&recite_core::AvailabilityReasonId>,
    context: &dyn DialogueContext,
    locale: LocaleLookup<'_>,
) -> Result<ChoiceAvailability, DialogueError> {
    let Some(requirement) = requirement else {
        return Ok(ChoiceAvailability::available());
    };

    let (is_available, reason_tree) = evaluate_availability_expression(
        asset,
        context,
        requirement,
        requirement_source_text,
        0,
        locale,
    )?;
    if is_available {
        return Ok(ChoiceAvailability::available());
    }

    Ok(ChoiceAvailability::unavailable(
        primary_reason_override.and_then(|reason| {
            reason_for_id(
                asset,
                reason,
                locale,
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
    locale: LocaleLookup<'_>,
) -> Result<(bool, Option<ChoiceAvailabilityReasonTree>), DialogueError> {
    if depth > MAX_AVAILABILITY_CONDITION_DEPTH {
        return Err(DialogueError::ConditionDepthLimitExceeded {
            limit: MAX_AVAILABILITY_CONDITION_DEPTH,
        });
    }

    match expression {
        CompiledConditionExpression::Call(call) => {
            evaluate_availability_call(asset, context, call, requirement_source_text, locale)
        }
        CompiledConditionExpression::And(expressions) => evaluate_availability_group(
            asset,
            context,
            expressions,
            requirement_source_text,
            depth,
            locale,
            AvailabilityGroup::All,
        ),
        CompiledConditionExpression::Or(expressions) => evaluate_availability_group(
            asset,
            context,
            expressions,
            requirement_source_text,
            depth,
            locale,
            AvailabilityGroup::Any,
        ),
        CompiledConditionExpression::Not(expression) => {
            let (child_available, _) = evaluate_availability_expression(
                asset,
                context,
                expression,
                requirement_source_text,
                depth + 1,
                locale,
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
    locale: LocaleLookup<'_>,
    group: AvailabilityGroup,
) -> Result<(bool, Option<ChoiceAvailabilityReasonTree>), DialogueError> {
    if expressions.is_empty() {
        let operator = match group {
            AvailabilityGroup::All => "and",
            AvailabilityGroup::Any => "or",
        };
        return Err(malformed(format!(
            "condition `{operator}` expression has no children"
        )));
    }

    let mut failed_children = Vec::new();
    let mut any_failed = false;
    for expression in expressions {
        let (is_available, child_tree) = evaluate_availability_expression(
            asset,
            context,
            expression,
            requirement_source_text,
            depth + 1,
            locale,
        )?;
        if !is_available {
            any_failed = true;
            if let Some(child_tree) = child_tree {
                failed_children.push(child_tree);
            }
        } else if matches!(group, AvailabilityGroup::Any) {
            return Ok((true, None));
        }
    }

    let is_available = !any_failed;
    let reason_tree = (!is_available && !failed_children.is_empty()).then(|| match group {
        AvailabilityGroup::All => ChoiceAvailabilityReasonTree::All(failed_children),
        AvailabilityGroup::Any => ChoiceAvailabilityReasonTree::Any(failed_children),
    });
    Ok((is_available, reason_tree))
}

fn evaluate_availability_call(
    asset: AssetView<'_>,
    context: &dyn DialogueContext,
    call: &CompiledConditionCall,
    requirement_source_text: Option<&str>,
    locale: LocaleLookup<'_>,
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
        reason_for_call(asset, call, locale)
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
    locale: LocaleLookup<'_>,
) -> Option<ChoiceAvailabilityReason> {
    let mapping = asset.condition_availability_reason(&call.function)?;
    reason_for_id_with_args(
        asset,
        &mapping.reason,
        locale,
        Some(ChoiceAvailabilityReasonOrigin::ConditionCall {
            function: call.function.clone(),
            args: call.args.iter().map(availability_argument).collect(),
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
    locale: LocaleLookup<'_>,
    origin: Option<ChoiceAvailabilityReasonOrigin>,
) -> Option<ChoiceAvailabilityReason> {
    reason_for_id_with_args(asset, reason_id, locale, origin, std::iter::empty())
}

fn reason_for_id_with_args(
    asset: AssetView<'_>,
    reason_id: &recite_core::AvailabilityReasonId,
    locale: LocaleLookup<'_>,
    origin: Option<ChoiceAvailabilityReasonOrigin>,
    args: impl IntoIterator<Item = ChoiceAvailabilityReasonArg>,
) -> Option<ChoiceAvailabilityReason> {
    let reason = asset.availability_reason(reason_id)?;
    let args = args.into_iter().collect::<Vec<_>>();
    let source_text = reason.template.clone();
    let localized_template = localise_reason_template(reason.id.as_str(), &source_text, locale);
    let text = render_reason_template(&localized_template, &args);
    Some(ChoiceAvailabilityReason {
        id: reason.id.clone(),
        source_text,
        text,
        origin,
        args,
    })
}

fn reason_arg_value(
    value: &CompiledAvailabilityReasonArgValue,
    call: &CompiledConditionCall,
) -> Option<ChoiceAvailabilityReasonValue> {
    match value {
        CompiledAvailabilityReasonArgValue::ConditionArg(name) => {
            condition_argument_value(name, &call.args)
        }
        CompiledAvailabilityReasonArgValue::LiteralString(value) => {
            Some(ChoiceAvailabilityReasonValue::String(value.clone()))
        }
        CompiledAvailabilityReasonArgValue::LiteralInt(value) => {
            Some(ChoiceAvailabilityReasonValue::Integer(*value))
        }
        CompiledAvailabilityReasonArgValue::LiteralFloat(value) => {
            Some(ChoiceAvailabilityReasonValue::Float(*value))
        }
        CompiledAvailabilityReasonArgValue::LiteralBool(value) => {
            Some(ChoiceAvailabilityReasonValue::Boolean(*value))
        }
    }
}

fn condition_argument_value(
    name: &str,
    args: &[CompiledArgument],
) -> Option<ChoiceAvailabilityReasonValue> {
    let index = name.parse::<usize>().ok()?;
    args.get(index).map(availability_argument)
}

fn availability_argument(argument: &CompiledArgument) -> ChoiceAvailabilityReasonValue {
    match argument {
        CompiledArgument::Identifier(value) => {
            ChoiceAvailabilityReasonValue::Identifier(value.clone())
        }
        CompiledArgument::Value(ScalarValue::String(value)) => {
            ChoiceAvailabilityReasonValue::String(value.clone())
        }
        CompiledArgument::Value(ScalarValue::Integer(value)) => {
            ChoiceAvailabilityReasonValue::Integer(*value)
        }
        CompiledArgument::Value(ScalarValue::Float(value)) => {
            ChoiceAvailabilityReasonValue::Float(*value)
        }
        CompiledArgument::Value(ScalarValue::Boolean(value)) => {
            ChoiceAvailabilityReasonValue::Boolean(*value)
        }
    }
}

fn localise_reason_template(id: &str, source_text: &str, locale: LocaleLookup<'_>) -> String {
    let Some((locale_id, provider)) = locale.locale.zip(locale.provider) else {
        return source_text.to_owned();
    };

    provider
        .lookup(
            id,
            source_text,
            TextDomain::AvailabilityReason,
            locale_id,
            locale.variant,
        )
        .unwrap_or_else(|| source_text.to_owned())
}

fn render_reason_template(template: &str, args: &[ChoiceAvailabilityReasonArg]) -> String {
    let mut rendered = template.to_owned();
    for arg in args {
        rendered = rendered.replace(
            &format!("{{{}}}", arg.name),
            &availability_value_text(&arg.value),
        );
    }
    rendered
}

fn availability_value_text(value: &ChoiceAvailabilityReasonValue) -> String {
    match value {
        ChoiceAvailabilityReasonValue::Identifier(value)
        | ChoiceAvailabilityReasonValue::String(value) => value.clone(),
        ChoiceAvailabilityReasonValue::Integer(value) => value.to_string(),
        ChoiceAvailabilityReasonValue::Float(value) => value.to_string(),
        ChoiceAvailabilityReasonValue::Boolean(value) => value.to_string(),
    }
}
