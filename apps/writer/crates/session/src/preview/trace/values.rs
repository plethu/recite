use super::Field;
use recite_runtime::{
    DialogueEffectArgument, DialogueEffectRequest,
    preview::{PreviewConditionArgument, PreviewConditionQuery},
};

pub fn condition(query: &PreviewConditionQuery) -> String {
    let arguments = query
        .arguments()
        .iter()
        .map(|argument| match argument {
            PreviewConditionArgument::Identifier(value) => value.clone(),
            PreviewConditionArgument::String(value) => format!("{value:?}"),
            PreviewConditionArgument::Integer(value) => value.to_string(),
            PreviewConditionArgument::Float(value) => value.to_string(),
            PreviewConditionArgument::Boolean(value) => value.to_string(),
            _ => "Unsupported argument".into(),
        })
        .collect::<Vec<_>>();
    format!("{}({})", query.function(), arguments.join(", "))
}
pub fn effect(effect: &DialogueEffectRequest) -> String {
    let arguments = effect
        .args
        .iter()
        .map(|argument| match argument {
            DialogueEffectArgument::Identifier(value) => value.clone(),
            DialogueEffectArgument::String(value) => format!("{value:?}"),
            DialogueEffectArgument::Integer(value) => value.to_string(),
            DialogueEffectArgument::Float(value) => value.to_string(),
            DialogueEffectArgument::Boolean(value) => value.to_string(),
        })
        .collect::<Vec<_>>();
    format!("{}({})", effect.function, arguments.join(", "))
}
pub(super) fn condition_argument(argument: &PreviewConditionArgument) -> Field<'_> {
    match argument {
        PreviewConditionArgument::Identifier(value) => Field::text("Identifier", value.as_str()),
        PreviewConditionArgument::String(value) => Field::text("String", value.as_str()),
        PreviewConditionArgument::Integer(value) => Field::display("Integer", value),
        PreviewConditionArgument::Float(value) => Field::display("Float", value),
        PreviewConditionArgument::Boolean(value) => Field::display("Boolean", value),
        _ => Field::text("Argument", "Unsupported argument"),
    }
}
pub(super) fn effect_argument(argument: &DialogueEffectArgument) -> Field<'_> {
    match argument {
        DialogueEffectArgument::Identifier(value) => Field::text("Identifier", value.as_str()),
        DialogueEffectArgument::String(value) => Field::text("String", value.as_str()),
        DialogueEffectArgument::Integer(value) => Field::display("Integer", value),
        DialogueEffectArgument::Float(value) => Field::display("Float", value),
        DialogueEffectArgument::Boolean(value) => Field::display("Boolean", value),
    }
}
