use recite_runtime::{ChoiceAvailabilityReasonValue, ConditionArgument, DialogueEffectArgument};
use serde::Serialize;

/// The shared named-map representation for condition, effect and reason values.
/// ABI-v0 tags and field order are independent of compiled-asset encoding.
#[derive(Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub(crate) enum TaggedValue {
    Identifier { value: String },
    String { value: String },
    Integer { value: i64 },
    Float { value: f64 },
    Boolean { value: bool },
}

impl From<ConditionArgument<'_>> for TaggedValue {
    fn from(argument: ConditionArgument<'_>) -> Self {
        match argument {
            ConditionArgument::Identifier(value) => Self::Identifier {
                value: value.to_owned(),
            },
            ConditionArgument::String(value) => Self::String {
                value: value.to_owned(),
            },
            ConditionArgument::Integer(value) => Self::Integer { value },
            ConditionArgument::Float(value) => Self::Float { value },
            ConditionArgument::Boolean(value) => Self::Boolean { value },
        }
    }
}

impl From<DialogueEffectArgument> for TaggedValue {
    fn from(argument: DialogueEffectArgument) -> Self {
        match argument {
            DialogueEffectArgument::Identifier(value) => Self::Identifier { value },
            DialogueEffectArgument::String(value) => Self::String { value },
            DialogueEffectArgument::Integer(value) => Self::Integer { value },
            DialogueEffectArgument::Float(value) => Self::Float { value },
            DialogueEffectArgument::Boolean(value) => Self::Boolean { value },
        }
    }
}

impl From<ChoiceAvailabilityReasonValue> for TaggedValue {
    fn from(value: ChoiceAvailabilityReasonValue) -> Self {
        match value {
            ChoiceAvailabilityReasonValue::Identifier(value) => Self::Identifier { value },
            ChoiceAvailabilityReasonValue::String(value) => Self::String { value },
            ChoiceAvailabilityReasonValue::Integer(value) => Self::Integer { value },
            ChoiceAvailabilityReasonValue::Float(value) => Self::Float { value },
            ChoiceAvailabilityReasonValue::Boolean(value) => Self::Boolean { value },
        }
    }
}
