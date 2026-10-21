use super::{Field, error, values};
use recite_runtime::{
    ConditionExpectedType, ConditionValue, DialogueEffectMode, EffectAck,
    preview::{PreviewConditionQuery, PreviewConditionResult, PreviewEvent},
};

pub(super) fn subject(event: &PreviewEvent) -> &str {
    match event {
        PreviewEvent::ConditionRequested(request)
        | PreviewEvent::ConditionResult { request, .. } => request.query().function(),
        PreviewEvent::EffectRequested(effect) | PreviewEvent::DeferredEffectScheduled(effect) => {
            &effect.function
        }
        PreviewEvent::Line(line) => &line.text,
        PreviewEvent::ChoiceAccepted { choice_id, .. }
        | PreviewEvent::ChoiceSelected { choice_id, .. } => choice_id.as_str(),
        PreviewEvent::Prompt(prompt) => prompt.line().map_or("", |line| &line.text),
        _ => "",
    }
}

pub(super) fn caption(event: &PreviewEvent) -> &'static str {
    match event {
        PreviewEvent::ConditionRequested(_) => "Condition requested",
        PreviewEvent::ConditionResult { .. } => "Condition result",
        PreviewEvent::Line(_) => "Line delivered",
        PreviewEvent::Prompt(_) => "Choices offered",
        PreviewEvent::ChoiceAccepted { .. } => "Choice accepted; traversal pending",
        PreviewEvent::ChoiceSelected { .. } => "Choice selected",
        PreviewEvent::EffectRequested(_) => "Effect requested",
        PreviewEvent::DeferredEffectScheduled(_) => "Deferred effect scheduled",
        PreviewEvent::EffectAcknowledged { .. } => "Effect acknowledged",
        PreviewEvent::End { .. } => "Conversation ended",
        PreviewEvent::Restarted { .. } => "Preview restarted",
        PreviewEvent::Restored => "Preview restored",
        PreviewEvent::RestartRequired { .. } => "Restart required",
        PreviewEvent::Error(_) => "Preview error",
        _ => "Unsupported trace event",
    }
}
pub(super) fn fields(event: &PreviewEvent) -> usize {
    1 + match event {
        PreviewEvent::ConditionRequested(request) => 3 + request.query().arguments().len(),
        PreviewEvent::ConditionResult { request, .. } => 4 + request.query().arguments().len(),
        PreviewEvent::EffectRequested(effect) | PreviewEvent::DeferredEffectScheduled(effect) => {
            3 + effect.args.len()
        }
        PreviewEvent::ChoiceAccepted { .. } | PreviewEvent::ChoiceSelected { .. } => 3,
        PreviewEvent::Line(_) => 4,
        PreviewEvent::Prompt(prompt) => 2 + prompt.choices().len() * 3,
        PreviewEvent::EffectAcknowledged { .. }
        | PreviewEvent::Restarted { .. }
        | PreviewEvent::RestartRequired { .. } => 2,
        PreviewEvent::End { .. } => 1,
        PreviewEvent::Error(error) => error::fields(error).len(),
        _ => 0,
    }
}
pub(super) fn field(event: &PreviewEvent, index: usize) -> Option<Field<'_>> {
    if index == 0 {
        return Some(Field::text("Event", caption(event)));
    }
    let index = index - 1;
    match event {
        PreviewEvent::ConditionRequested(request) => {
            query(request.query(), request.block().as_str(), index)
        }
        PreviewEvent::ConditionResult { request, result } => {
            if index == 0 {
                Some(condition_result(result))
            } else {
                query(request.query(), request.block().as_str(), index - 1)
            }
        }
        PreviewEvent::EffectRequested(effect) | PreviewEvent::DeferredEffectScheduled(effect) => {
            Some(match index {
                0 => Field::text("Effect ID", effect.id.as_str()),
                1 => Field::text(
                    "Effect mode",
                    match effect.mode {
                        DialogueEffectMode::Immediate => "Immediate",
                        DialogueEffectMode::Deferred => "Deferred",
                        DialogueEffectMode::Blocking => "Blocking",
                    },
                ),
                2 => Field::text("Function", effect.function.as_str()),
                index => values::effect_argument(effect.args.get(index - 3)?).argument(index - 2),
            })
        }
        PreviewEvent::ChoiceAccepted { prompt, choice_id }
        | PreviewEvent::ChoiceSelected { prompt, choice_id } => Some(match index {
            0 => Field::text("Choice ID", choice_id.as_str()),
            1 => Field::text("Beat", prompt.block().as_str()),
            2 => Field::optional("Prompt line ID", prompt.line().map(|line| line.as_str())),
            _ => return None,
        }),
        PreviewEvent::Line(line) => Some(match index {
            0 => Field::text("Line ID", line.id.as_str()),
            1 => Field::text("Source text", line.source_text.as_str()),
            2 => Field::text("Delivered text", line.text.as_str()),
            3 => Field::optional(
                "Speaker",
                line.speaker.as_ref().map(|speaker| speaker.as_str()),
            ),
            _ => return None,
        }),
        PreviewEvent::Prompt(prompt) => Some(match index {
            0 => Field::text("Beat", prompt.identity().block().as_str()),
            1 => Field::optional("Prompt text", prompt.line().map(|line| line.text.as_str())),
            index => {
                let index = index - 2;
                let choice = prompt.choices().get(index / 3)?;
                match index % 3 {
                    0 => Field::text("Choice ID", choice.id.as_str()),
                    1 => Field::text("Choice text", choice.text.as_str()),
                    _ => Field::display("Choice available", choice.availability.is_available),
                }
            }
        }),
        PreviewEvent::EffectAcknowledged { effect_id, ack } => Some(match index {
            0 => Field::text("Effect ID", effect_id.as_str()),
            1 => match ack {
                EffectAck::Completed => Field::text("Acknowledgement", "Completed"),
                EffectAck::Failed { reason } => Field::text("Failure", reason.as_str()),
            },
            _ => return None,
        }),
        PreviewEvent::End { deferred_effects } if index == 0 => {
            Some(Field::display("Deferred effects", deferred_effects.len()))
        }
        PreviewEvent::Restarted { block, locale } => Some(match index {
            0 => Field::optional("Beat", block.as_ref().map(|block| block.as_str())),
            1 => Field::optional("Language", locale.as_ref().map(|locale| locale.as_str())),
            _ => return None,
        }),
        PreviewEvent::RestartRequired {
            active_asset,
            replacement_asset,
            ..
        } => Some(match index {
            0 => Field::text("Active asset", active_asset.as_str()),
            1 => Field::text("Replacement asset", replacement_asset.as_str()),
            _ => return None,
        }),
        PreviewEvent::Error(error) => error::fields(error).into_iter().nth(index),
        _ => None,
    }
}
fn query<'a>(query: &'a PreviewConditionQuery, block: &'a str, index: usize) -> Option<Field<'a>> {
    Some(match index {
        0 => Field::text("Function", query.function()),
        1 => Field::text(
            "Expected result",
            match query.expected_type() {
                ConditionExpectedType::Bool => "Boolean",
                ConditionExpectedType::Enum => "Enum variant",
            },
        ),
        2 => Field::text("Beat", block),
        index => values::condition_argument(query.arguments().get(index - 3)?).argument(index - 2),
    })
}
fn condition_result(result: &PreviewConditionResult) -> Field<'_> {
    match result {
        PreviewConditionResult::Value(ConditionValue::Bool(value)) => {
            Field::display("Boolean result", value)
        }
        PreviewConditionResult::Value(ConditionValue::EnumVariant(value)) => {
            Field::text("Enum result", value.as_str())
        }
        PreviewConditionResult::Failed { reason } => Field::text("Failure", reason.as_str()),
        _ => Field::text("Result", "Unsupported result"),
    }
}
