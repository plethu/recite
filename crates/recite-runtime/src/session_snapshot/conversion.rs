use recite_core::{AvailabilityReasonId, CoreValueError};

use crate::event::{
    ChoiceAvailability, ChoiceAvailabilityReason, ChoiceAvailabilityReasonArg,
    ChoiceAvailabilityReasonOrigin, ChoiceAvailabilityReasonTree, ChoiceAvailabilityReasonValue,
};

use super::{
    DialogueChoiceAvailabilityReasonArgSnapshot, DialogueChoiceAvailabilityReasonOriginSnapshot,
    DialogueChoiceAvailabilityReasonSnapshot, DialogueChoiceAvailabilityReasonTreeSnapshot,
    DialogueChoiceAvailabilityReasonValueSnapshot, DialogueChoiceAvailabilitySnapshot,
};

/// A failure while converting a trusted snapshot value into runtime state.
///
/// Snapshot restoration adds the context that is lost when a core value
/// constructor is converted directly into a display string. The enclosing
/// restore operation may still adapt this error to its public runtime error
/// category, but conversion itself remains typed and inspectable.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
pub(crate) enum DialogueSessionSnapshotConversionError {
    #[error("invalid availability reason id `{id}`: {source}")]
    InvalidAvailabilityReasonId {
        id: String,
        #[source]
        source: CoreValueError,
    },
}

pub(crate) fn availability_snapshot(
    availability: &ChoiceAvailability,
) -> DialogueChoiceAvailabilitySnapshot {
    DialogueChoiceAvailabilitySnapshot {
        is_available: availability.is_available,
        primary_reason: availability.primary_reason.as_ref().map(reason_snapshot),
        reason_tree: availability.reason_tree.as_ref().map(reason_tree_snapshot),
    }
}

pub(crate) fn availability_from_snapshot(
    snapshot: DialogueChoiceAvailabilitySnapshot,
) -> Result<ChoiceAvailability, DialogueSessionSnapshotConversionError> {
    Ok(ChoiceAvailability {
        is_available: snapshot.is_available,
        primary_reason: snapshot
            .primary_reason
            .map(reason_from_snapshot)
            .transpose()?,
        reason_tree: snapshot
            .reason_tree
            .map(reason_tree_from_snapshot)
            .transpose()?,
    })
}

fn reason_snapshot(reason: &ChoiceAvailabilityReason) -> DialogueChoiceAvailabilityReasonSnapshot {
    DialogueChoiceAvailabilityReasonSnapshot {
        id: reason.id.as_str().to_owned(),
        source_text: reason.source_text.clone(),
        text: reason.text.clone(),
        origin: reason.origin.as_ref().map(reason_origin_snapshot),
        args: reason
            .args
            .iter()
            .map(|arg| DialogueChoiceAvailabilityReasonArgSnapshot {
                name: arg.name.clone(),
                value: reason_value_snapshot(&arg.value),
            })
            .collect(),
    }
}

fn reason_from_snapshot(
    snapshot: DialogueChoiceAvailabilityReasonSnapshot,
) -> Result<ChoiceAvailabilityReason, DialogueSessionSnapshotConversionError> {
    let id = snapshot.id.clone();
    let reason_id = AvailabilityReasonId::new(snapshot.id).map_err(|source| {
        DialogueSessionSnapshotConversionError::InvalidAvailabilityReasonId { id, source }
    })?;

    Ok(ChoiceAvailabilityReason {
        id: reason_id,
        source_text: snapshot.source_text,
        text: snapshot.text,
        origin: snapshot.origin.map(reason_origin_from_snapshot),
        args: snapshot
            .args
            .into_iter()
            .map(|arg| ChoiceAvailabilityReasonArg {
                name: arg.name,
                value: reason_value_from_snapshot(arg.value),
            })
            .collect(),
    })
}

fn reason_origin_snapshot(
    origin: &ChoiceAvailabilityReasonOrigin,
) -> DialogueChoiceAvailabilityReasonOriginSnapshot {
    match origin {
        ChoiceAvailabilityReasonOrigin::ConditionCall { function, args } => {
            DialogueChoiceAvailabilityReasonOriginSnapshot::ConditionCall {
                function: function.clone(),
                args: args.iter().map(reason_value_snapshot).collect(),
            }
        }
        ChoiceAvailabilityReasonOrigin::RequirementExpression { source_text } => {
            DialogueChoiceAvailabilityReasonOriginSnapshot::RequirementExpression {
                source_text: source_text.clone(),
            }
        }
    }
}

fn reason_origin_from_snapshot(
    snapshot: DialogueChoiceAvailabilityReasonOriginSnapshot,
) -> ChoiceAvailabilityReasonOrigin {
    match snapshot {
        DialogueChoiceAvailabilityReasonOriginSnapshot::ConditionCall { function, args } => {
            ChoiceAvailabilityReasonOrigin::ConditionCall {
                function,
                args: args.into_iter().map(reason_value_from_snapshot).collect(),
            }
        }
        DialogueChoiceAvailabilityReasonOriginSnapshot::RequirementExpression { source_text } => {
            ChoiceAvailabilityReasonOrigin::RequirementExpression { source_text }
        }
    }
}

fn reason_value_snapshot(
    value: &ChoiceAvailabilityReasonValue,
) -> DialogueChoiceAvailabilityReasonValueSnapshot {
    match value {
        ChoiceAvailabilityReasonValue::Identifier(value) => {
            DialogueChoiceAvailabilityReasonValueSnapshot::Identifier(value.clone())
        }
        ChoiceAvailabilityReasonValue::String(value) => {
            DialogueChoiceAvailabilityReasonValueSnapshot::String(value.clone())
        }
        ChoiceAvailabilityReasonValue::Integer(value) => {
            DialogueChoiceAvailabilityReasonValueSnapshot::Integer(*value)
        }
        ChoiceAvailabilityReasonValue::Float(value) => {
            DialogueChoiceAvailabilityReasonValueSnapshot::Float(*value)
        }
        ChoiceAvailabilityReasonValue::Boolean(value) => {
            DialogueChoiceAvailabilityReasonValueSnapshot::Boolean(*value)
        }
    }
}

fn reason_value_from_snapshot(
    snapshot: DialogueChoiceAvailabilityReasonValueSnapshot,
) -> ChoiceAvailabilityReasonValue {
    match snapshot {
        DialogueChoiceAvailabilityReasonValueSnapshot::Identifier(value) => {
            ChoiceAvailabilityReasonValue::Identifier(value)
        }
        DialogueChoiceAvailabilityReasonValueSnapshot::String(value) => {
            ChoiceAvailabilityReasonValue::String(value)
        }
        DialogueChoiceAvailabilityReasonValueSnapshot::Integer(value) => {
            ChoiceAvailabilityReasonValue::Integer(value)
        }
        DialogueChoiceAvailabilityReasonValueSnapshot::Float(value) => {
            ChoiceAvailabilityReasonValue::Float(value)
        }
        DialogueChoiceAvailabilityReasonValueSnapshot::Boolean(value) => {
            ChoiceAvailabilityReasonValue::Boolean(value)
        }
    }
}

fn reason_tree_snapshot(
    tree: &ChoiceAvailabilityReasonTree,
) -> DialogueChoiceAvailabilityReasonTreeSnapshot {
    match tree {
        ChoiceAvailabilityReasonTree::All(children) => {
            DialogueChoiceAvailabilityReasonTreeSnapshot::All(
                children.iter().map(reason_tree_snapshot).collect(),
            )
        }
        ChoiceAvailabilityReasonTree::Any(children) => {
            DialogueChoiceAvailabilityReasonTreeSnapshot::Any(
                children.iter().map(reason_tree_snapshot).collect(),
            )
        }
        ChoiceAvailabilityReasonTree::Reason(reason) => {
            DialogueChoiceAvailabilityReasonTreeSnapshot::Reason(reason_snapshot(reason))
        }
        ChoiceAvailabilityReasonTree::RequirementSourceText(text) => {
            DialogueChoiceAvailabilityReasonTreeSnapshot::RequirementSourceText(text.clone())
        }
    }
}

fn reason_tree_from_snapshot(
    snapshot: DialogueChoiceAvailabilityReasonTreeSnapshot,
) -> Result<ChoiceAvailabilityReasonTree, DialogueSessionSnapshotConversionError> {
    match snapshot {
        DialogueChoiceAvailabilityReasonTreeSnapshot::All(children) => {
            Ok(ChoiceAvailabilityReasonTree::All(
                children
                    .into_iter()
                    .map(reason_tree_from_snapshot)
                    .collect::<Result<Vec<_>, _>>()?,
            ))
        }
        DialogueChoiceAvailabilityReasonTreeSnapshot::Any(children) => {
            Ok(ChoiceAvailabilityReasonTree::Any(
                children
                    .into_iter()
                    .map(reason_tree_from_snapshot)
                    .collect::<Result<Vec<_>, _>>()?,
            ))
        }
        DialogueChoiceAvailabilityReasonTreeSnapshot::Reason(reason) => Ok(
            ChoiceAvailabilityReasonTree::Reason(reason_from_snapshot(reason)?),
        ),
        DialogueChoiceAvailabilityReasonTreeSnapshot::RequirementSourceText(text) => {
            Ok(ChoiceAvailabilityReasonTree::RequirementSourceText(text))
        }
    }
}
