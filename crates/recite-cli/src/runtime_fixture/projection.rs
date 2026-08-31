use recite_runtime::{
    ConditionValue, PreviewConditionArgument, PreviewConditionRequest, PreviewConditionResult,
    PreviewEvent, PreviewPrompt, PreviewTrace,
};
use recite_ui::UiArg;

use super::metrics::RuntimeMetricsCollector;
use super::prompt::write_prompt_run_lines;
use super::trace::{
    TraceCondition, TraceConditionValue, TraceEvent, TracePrompt, TracePromptIdentity, TraceScalar,
    condition_query_text, trace_choice, trace_effect, trace_line,
};
use crate::error::CliError;
use crate::i18n::{Messages, MsgId};

pub(crate) struct RuntimeExecution {
    pub(crate) run_lines: Vec<String>,
    pub(crate) trace: super::trace::TraceDocument,
}

pub(super) fn project_event(
    event: &PreviewEvent,
    preview_trace: &PreviewTrace,
    run_lines: &mut Vec<String>,
    trace_events: &mut Vec<TraceEvent>,
    metrics: Option<&mut RuntimeMetricsCollector>,
    messages: &Messages,
) -> Result<(), CliError> {
    let mut metrics = metrics;
    match event {
        PreviewEvent::ConditionResult { request, result } => {
            let PreviewConditionResult::Value(value) = result else {
                return Ok(());
            };
            let query = trace_condition_query(request)?;
            let condition = TraceCondition {
                query: query.clone(),
                function: request.query().function().to_owned(),
                arguments: request
                    .query()
                    .arguments()
                    .iter()
                    .map(trace_preview_condition_argument)
                    .collect::<Result<Vec<_>, _>>()?,
                result: trace_condition_value(value),
            };
            run_lines.push(messages.format(
                MsgId::PlayConditionResult,
                [
                    ("query", UiArg::from(query)),
                    ("result", UiArg::from(condition.result.to_string())),
                ],
            ));
            trace_events.push(TraceEvent::Condition { condition });
            if let Some(metrics) = metrics.as_deref_mut() {
                metrics.condition_evaluation_count += 1;
            }
        }
        PreviewEvent::Line(line) => {
            run_lines.push(messages.format(
                MsgId::PlayLine,
                [
                    ("id", UiArg::from(line.id.as_str())),
                    ("text", UiArg::from(line.text.as_str())),
                ],
            ));
            if let Some(metrics) = metrics.as_deref_mut() {
                metrics.line_count += 1;
            }
            trace_events.push(TraceEvent::Line {
                line: trace_line(line, preview_trace),
            });
        }
        PreviewEvent::Prompt(prompt) => {
            write_prompt_run_lines(run_lines, prompt.line(), prompt.choices(), messages);
            if let Some(metrics) = metrics.as_deref_mut() {
                metrics.prompt_count += 1;
                metrics.choice_count += prompt.choices().len();
                if prompt.line().is_some() {
                    metrics.line_count += 1;
                }
            }
            trace_events.push(TraceEvent::Prompt {
                prompt: trace_prompt(prompt, preview_trace),
            });
        }
        PreviewEvent::ChoiceSelected { prompt, choice_id } => {
            run_lines.push(messages.format(
                MsgId::PlaySelectedChoice,
                [("id", UiArg::from(choice_id.as_str()))],
            ));
            trace_events.push(TraceEvent::ChoiceSelected {
                prompt: TracePromptIdentity {
                    block: prompt.block().as_str().to_owned(),
                    line: prompt.line().map(|line| line.as_str().to_owned()),
                    fixture_keys: fixture_keys(prompt),
                },
                choice: choice_id.as_str().to_owned(),
            });
        }
        PreviewEvent::EffectRequested(effect) => {
            run_lines.push(messages.format(
                MsgId::RunEffect,
                [
                    ("mode", UiArg::from(effect.mode.to_string())),
                    ("function", UiArg::from(effect.function.clone())),
                    (
                        "args",
                        UiArg::from(super::trace::format_effect_arguments(&effect.args)),
                    ),
                ],
            ));
            if let Some(metrics) = metrics.as_deref_mut() {
                metrics.record_effect(effect.mode);
            }
            trace_events.push(TraceEvent::Effect {
                effect: trace_effect(effect),
            });
        }
        PreviewEvent::EffectAcknowledged { effect_id, .. } => {
            run_lines.push(messages.format(
                MsgId::PlayAckCompleted,
                [("id", UiArg::from(effect_id.as_str()))],
            ));
            trace_events.push(TraceEvent::Acknowledgement {
                effect_id: effect_id.as_str().to_owned(),
                result: "completed",
            });
        }
        PreviewEvent::End { deferred_effects } => {
            run_lines.push(messages.text(MsgId::PlayEnd));
            if !deferred_effects.is_empty() {
                run_lines.push(messages.text(MsgId::PlayDeferredEffects));
                for effect in deferred_effects {
                    run_lines.push(messages.format(
                        MsgId::PlayDeferredEffectRow,
                        [
                            ("function", UiArg::from(effect.function.clone())),
                            (
                                "args",
                                UiArg::from(super::trace::format_effect_arguments(&effect.args)),
                            ),
                        ],
                    ));
                    if let Some(metrics) = metrics.as_deref_mut() {
                        metrics.record_effect(effect.mode);
                    }
                }
            }
            trace_events.push(TraceEvent::End {
                deferred_effects: deferred_effects.iter().map(trace_effect).collect(),
            });
        }
        PreviewEvent::ConditionRequested(_)
        | PreviewEvent::DeferredEffectScheduled(_)
        | PreviewEvent::Restarted { .. }
        | PreviewEvent::Restored
        | PreviewEvent::RestartRequired { .. }
        | PreviewEvent::Error(_) => {}
        _ => {
            return Err(CliError::MalformedCompiledAsset {
                reason: "preview emitted an unsupported structured event".to_owned(),
            });
        }
    }
    Ok(())
}

fn trace_prompt(prompt: &PreviewPrompt, preview_trace: &PreviewTrace) -> TracePrompt {
    let identity = prompt.identity();
    TracePrompt {
        identity: TracePromptIdentity {
            block: identity.block().as_str().to_owned(),
            line: identity.line().map(|line| line.as_str().to_owned()),
            fixture_keys: fixture_keys(identity),
        },
        line: prompt.line().map(|line| trace_line(line, preview_trace)),
        choices: prompt
            .choices()
            .iter()
            .map(|choice| trace_choice(choice, preview_trace))
            .collect(),
    }
}

fn fixture_keys(identity: &recite_runtime::PreviewPromptIdentity) -> Vec<String> {
    let mut keys = identity
        .line()
        .map(|line| vec![line.as_str().to_owned()])
        .unwrap_or_default();
    keys.push(identity.block().as_str().to_owned());
    keys
}

fn trace_condition_query(request: &PreviewConditionRequest) -> Result<String, CliError> {
    Ok(condition_query_text(
        request.query().function(),
        &request
            .query()
            .arguments()
            .iter()
            .map(trace_preview_condition_argument)
            .collect::<Result<Vec<_>, _>>()?,
    ))
}

fn trace_preview_condition_argument(
    argument: &PreviewConditionArgument,
) -> Result<TraceScalar, CliError> {
    let value = match argument {
        PreviewConditionArgument::Identifier(value) => TraceScalar::Identifier(value.clone()),
        PreviewConditionArgument::String(value) => TraceScalar::String(value.clone()),
        PreviewConditionArgument::Integer(value) => TraceScalar::Integer(*value),
        PreviewConditionArgument::Float(value) => TraceScalar::Float(*value),
        PreviewConditionArgument::Boolean(value) => TraceScalar::Boolean(*value),
        _ => {
            return Err(CliError::MalformedCompiledAsset {
                reason: "preview emitted an unsupported condition argument".to_owned(),
            });
        }
    };
    Ok(value)
}

fn trace_condition_value(value: &ConditionValue) -> TraceConditionValue {
    match value {
        ConditionValue::Bool(value) => TraceConditionValue::Bool(*value),
        ConditionValue::EnumVariant(value) => TraceConditionValue::EnumVariant {
            r#enum: value.clone(),
        },
    }
}
