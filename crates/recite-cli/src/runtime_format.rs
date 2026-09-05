use recite_runtime::{ConditionArgument, DialogueEffectArgument};

pub(crate) enum RuntimeDisplayArgument<'a> {
    Identifier(&'a str),
    String(&'a str),
    Integer(i64),
    Float(f64),
    Boolean(bool),
}

impl<'a> From<ConditionArgument<'a>> for RuntimeDisplayArgument<'a> {
    fn from(argument: ConditionArgument<'a>) -> Self {
        match argument {
            ConditionArgument::Identifier(value) => Self::Identifier(value),
            ConditionArgument::String(value) => Self::String(value),
            ConditionArgument::Integer(value) => Self::Integer(value),
            ConditionArgument::Float(value) => Self::Float(value),
            ConditionArgument::Boolean(value) => Self::Boolean(value),
        }
    }
}

impl<'a> From<&'a DialogueEffectArgument> for RuntimeDisplayArgument<'a> {
    fn from(argument: &'a DialogueEffectArgument) -> Self {
        match argument {
            DialogueEffectArgument::Identifier(value) => Self::Identifier(value),
            DialogueEffectArgument::String(value) => Self::String(value),
            DialogueEffectArgument::Integer(value) => Self::Integer(*value),
            DialogueEffectArgument::Float(value) => Self::Float(*value),
            DialogueEffectArgument::Boolean(value) => Self::Boolean(*value),
        }
    }
}

pub(crate) fn format_condition_query<'a>(
    function: &str,
    arguments: impl IntoIterator<Item = RuntimeDisplayArgument<'a>>,
) -> String {
    let arguments = arguments
        .into_iter()
        .map(format_runtime_argument)
        .collect::<Vec<_>>()
        .join(", ");
    format!("{function}({arguments})")
}

pub(crate) fn format_effect_arguments(arguments: &[DialogueEffectArgument]) -> String {
    let arguments = arguments
        .iter()
        .map(RuntimeDisplayArgument::from)
        .map(format_runtime_argument)
        .collect::<Vec<_>>()
        .join(", ");
    format!("({arguments})")
}

// Invariant: serde_json string serialization has no data-dependent failure path.
#[allow(
    clippy::expect_used,
    reason = "serde_json serialization of a borrowed string has no data-dependent failure"
)]
pub(crate) fn format_runtime_argument(argument: RuntimeDisplayArgument<'_>) -> String {
    match argument {
        RuntimeDisplayArgument::Identifier(value) => value.to_owned(),
        RuntimeDisplayArgument::String(value) => {
            serde_json::to_string(value).expect("serializing a string cannot fail")
        }
        RuntimeDisplayArgument::Integer(value) => value.to_string(),
        RuntimeDisplayArgument::Float(value) => value.to_string(),
        RuntimeDisplayArgument::Boolean(value) => value.to_string(),
    }
}
