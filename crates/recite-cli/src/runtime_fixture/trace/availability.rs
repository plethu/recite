use recite_runtime::{
    ChoiceAvailability, ChoiceAvailabilityReason, ChoiceAvailabilityReasonArg,
    ChoiceAvailabilityReasonOrigin, ChoiceAvailabilityReasonTree, ChoiceAvailabilityReasonValue,
    PreviewTrace, TextDomain,
};

use super::model::{
    TraceChoiceAvailability, TraceChoiceAvailabilityReason, TraceChoiceAvailabilityReasonArg,
    TraceChoiceAvailabilityReasonOrigin, TraceChoiceAvailabilityReasonTree,
    TraceChoiceAvailabilityReasonValue,
};

pub(super) fn trace_availability(
    availability: &ChoiceAvailability,
    dialogue_trace: &PreviewTrace,
) -> TraceChoiceAvailability {
    TraceChoiceAvailability {
        is_available: availability.is_available,
        primary_reason: availability
            .primary_reason
            .as_ref()
            .map(|reason| trace_availability_reason(reason, dialogue_trace)),
        reason_tree: availability
            .reason_tree
            .as_ref()
            .map(|tree| trace_availability_reason_tree(tree, dialogue_trace)),
    }
}

fn trace_availability_reason(
    reason: &ChoiceAvailabilityReason,
    dialogue_trace: &PreviewTrace,
) -> TraceChoiceAvailabilityReason {
    TraceChoiceAvailabilityReason {
        id: reason.id.as_str().to_owned(),
        source_text: reason.source_text.clone(),
        localized_template: localized_template(dialogue_trace, reason.id.as_str())
            .unwrap_or_else(|| reason.source_text.clone()),
        text: reason.text.clone(),
        origin: reason.origin.as_ref().map(trace_availability_reason_origin),
        args: reason
            .args
            .iter()
            .map(trace_availability_reason_arg)
            .collect(),
    }
}

fn trace_availability_reason_origin(
    origin: &ChoiceAvailabilityReasonOrigin,
) -> TraceChoiceAvailabilityReasonOrigin {
    match origin {
        ChoiceAvailabilityReasonOrigin::ConditionCall { function, args } => {
            TraceChoiceAvailabilityReasonOrigin::ConditionCall {
                function: function.clone(),
                args: args.iter().map(trace_availability_reason_value).collect(),
            }
        }
        ChoiceAvailabilityReasonOrigin::RequirementExpression { source_text } => {
            TraceChoiceAvailabilityReasonOrigin::RequirementExpression {
                source_text: source_text.clone(),
            }
        }
    }
}

fn trace_availability_reason_arg(
    arg: &ChoiceAvailabilityReasonArg,
) -> TraceChoiceAvailabilityReasonArg {
    TraceChoiceAvailabilityReasonArg {
        name: arg.name.clone(),
        value: trace_availability_reason_value(&arg.value),
    }
}

fn trace_availability_reason_value(
    value: &ChoiceAvailabilityReasonValue,
) -> TraceChoiceAvailabilityReasonValue {
    match value {
        ChoiceAvailabilityReasonValue::Identifier(value) => {
            TraceChoiceAvailabilityReasonValue::Identifier(value.clone())
        }
        ChoiceAvailabilityReasonValue::String(value) => {
            TraceChoiceAvailabilityReasonValue::String(value.clone())
        }
        ChoiceAvailabilityReasonValue::Integer(value) => {
            TraceChoiceAvailabilityReasonValue::Integer(*value)
        }
        ChoiceAvailabilityReasonValue::Float(value) => {
            TraceChoiceAvailabilityReasonValue::Float(*value)
        }
        ChoiceAvailabilityReasonValue::Boolean(value) => {
            TraceChoiceAvailabilityReasonValue::Boolean(*value)
        }
    }
}

fn trace_availability_reason_tree(
    tree: &ChoiceAvailabilityReasonTree,
    dialogue_trace: &PreviewTrace,
) -> TraceChoiceAvailabilityReasonTree {
    match tree {
        ChoiceAvailabilityReasonTree::All(children) => TraceChoiceAvailabilityReasonTree::All(
            children
                .iter()
                .map(|tree| trace_availability_reason_tree(tree, dialogue_trace))
                .collect(),
        ),
        ChoiceAvailabilityReasonTree::Any(children) => TraceChoiceAvailabilityReasonTree::Any(
            children
                .iter()
                .map(|tree| trace_availability_reason_tree(tree, dialogue_trace))
                .collect(),
        ),
        ChoiceAvailabilityReasonTree::Reason(reason) => TraceChoiceAvailabilityReasonTree::Reason(
            trace_availability_reason(reason, dialogue_trace),
        ),
        ChoiceAvailabilityReasonTree::RequirementSourceText(source_text) => {
            TraceChoiceAvailabilityReasonTree::RequirementSourceText(source_text.clone())
        }
    }
}

fn localized_template(trace: &PreviewTrace, id: &str) -> Option<String> {
    trace
        .localized_lookups()
        .filter(|lookup| lookup.id == id && lookup.domain == TextDomain::AvailabilityReason)
        .last()
        .and_then(|lookup| lookup.resolved_text.clone())
}
