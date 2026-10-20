use recite_runtime::preview::{PreviewConditionArgument, PreviewConditionQuery};
use recite_runtime::{DialogueEffectArgument, DialogueEffectMode, DialogueEffectRequest};

pub(super) fn condition(query: &PreviewConditionQuery) -> String {
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
    format!(
        "Condition input · {}({})",
        query.function(),
        arguments.join(", ")
    )
}

pub(super) fn effect(effect: &DialogueEffectRequest) -> String {
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
    let mode = match effect.mode {
        DialogueEffectMode::Deferred => "Deferred",
        DialogueEffectMode::Immediate => "Immediate",
        DialogueEffectMode::Blocking => "Blocking",
    };
    format!(
        "{mode} effect request · {}({})",
        effect.function,
        arguments.join(", ")
    )
}
