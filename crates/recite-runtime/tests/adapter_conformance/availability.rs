use recite_runtime::{
    ChoiceAvailabilityReason, ChoiceAvailabilityReasonOrigin, ChoiceAvailabilityReasonTree,
    ChoiceAvailabilityReasonValue, DialogueChoice,
};

use super::manifest::{
    AvailabilityReasonArgExpectation, AvailabilityReasonExpectation,
    AvailabilityReasonOriginExpectation, AvailabilityReasonTreeExpectation,
    AvailabilityReasonValueExpectation, AvailabilityReasonValueKind, ChoiceAvailabilityExpectation,
};

pub(crate) fn choice_availability_expectation(
    choice: &DialogueChoice,
) -> ChoiceAvailabilityExpectation {
    ChoiceAvailabilityExpectation {
        choice_id: choice.id.as_str().to_owned(),
        is_available: choice.availability.is_available,
        primary_reason: choice
            .availability
            .primary_reason
            .as_ref()
            .map(availability_reason_expectation),
        reason_tree: choice
            .availability
            .reason_tree
            .as_ref()
            .map(availability_reason_tree_expectation),
    }
}

fn availability_reason_tree_expectation(
    tree: &ChoiceAvailabilityReasonTree,
) -> AvailabilityReasonTreeExpectation {
    match tree {
        ChoiceAvailabilityReasonTree::All(children) => AvailabilityReasonTreeExpectation::All {
            children: children
                .iter()
                .map(availability_reason_tree_expectation)
                .collect(),
        },
        ChoiceAvailabilityReasonTree::Any(children) => AvailabilityReasonTreeExpectation::Any {
            children: children
                .iter()
                .map(availability_reason_tree_expectation)
                .collect(),
        },
        ChoiceAvailabilityReasonTree::Reason(reason) => AvailabilityReasonTreeExpectation::Reason {
            reason: availability_reason_expectation(reason),
        },
        ChoiceAvailabilityReasonTree::RequirementSourceText(source_text) => {
            AvailabilityReasonTreeExpectation::RequirementSourceText {
                source_text: source_text.clone(),
            }
        }
    }
}

fn availability_reason_expectation(
    reason: &ChoiceAvailabilityReason,
) -> AvailabilityReasonExpectation {
    AvailabilityReasonExpectation {
        id: reason.id.as_str().to_owned(),
        source_text: reason.source_text.clone(),
        text: reason.text.clone(),
        origin: reason.origin.as_ref().map(availability_reason_origin),
        args: reason
            .args
            .iter()
            .map(|arg| AvailabilityReasonArgExpectation {
                name: arg.name.clone(),
                value: availability_reason_value(&arg.value),
            })
            .collect(),
    }
}

fn availability_reason_origin(
    origin: &ChoiceAvailabilityReasonOrigin,
) -> AvailabilityReasonOriginExpectation {
    match origin {
        ChoiceAvailabilityReasonOrigin::ConditionCall { function, args } => {
            AvailabilityReasonOriginExpectation::ConditionCall {
                function: function.clone(),
                args: args.iter().map(availability_reason_value).collect(),
            }
        }
        ChoiceAvailabilityReasonOrigin::RequirementExpression { source_text } => {
            AvailabilityReasonOriginExpectation::RequirementExpression {
                source_text: source_text.clone(),
            }
        }
    }
}

fn availability_reason_value(
    value: &ChoiceAvailabilityReasonValue,
) -> AvailabilityReasonValueExpectation {
    match value {
        ChoiceAvailabilityReasonValue::Identifier(value) => AvailabilityReasonValueExpectation {
            kind: AvailabilityReasonValueKind::Identifier,
            value: value.clone().into(),
        },
        ChoiceAvailabilityReasonValue::String(value) => AvailabilityReasonValueExpectation {
            kind: AvailabilityReasonValueKind::String,
            value: value.clone().into(),
        },
        ChoiceAvailabilityReasonValue::Integer(value) => AvailabilityReasonValueExpectation {
            kind: AvailabilityReasonValueKind::Integer,
            value: (*value).into(),
        },
        ChoiceAvailabilityReasonValue::Float(value) => AvailabilityReasonValueExpectation {
            kind: AvailabilityReasonValueKind::Float,
            value: serde_json::json!(value),
        },
        ChoiceAvailabilityReasonValue::Boolean(value) => AvailabilityReasonValueExpectation {
            kind: AvailabilityReasonValueKind::Boolean,
            value: (*value).into(),
        },
    }
}
