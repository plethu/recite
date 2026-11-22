use recite_runtime::{
    ConditionValue,
    preview::{
        PreviewConditionArgument, PreviewConditionRequest, PreviewConditionResult, PreviewEvent,
        PreviewPrompt, PreviewTrace,
    },
};
use recite_ui::UiArg;

use super::metrics::RuntimeMetricsCollector;
use super::prompt::PromptCardinality;
use super::trace::{
    TraceCondition, TraceConditionValue, TraceEvent, TracePrompt, TracePromptIdentity, TraceScalar,
    condition_query_text, trace_choice, trace_effect, trace_line,
};
use super::transcript::{RunChoice, RunLine, RunTranscript};
use crate::error::CliError;
use crate::i18n::{Messages, MsgId};

pub(crate) struct RuntimeExecution {
    pub(crate) run: RunTranscript,
    pub(crate) trace: super::trace::TraceDocument,
}

pub(super) fn project_event(
    event: &PreviewEvent,
    preview_trace: &PreviewTrace,
    prompt_cardinality: &PromptCardinality,
    run: &mut RunTranscript,
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
            run.condition(query, condition.result.to_string());
            trace_events.push(TraceEvent::Condition { condition });
            if let Some(metrics) = metrics.as_deref_mut() {
                metrics.condition_evaluation_count += 1;
            }
        }
        PreviewEvent::Line(line) => {
            run.line(run_line(line));
            if let Some(metrics) = metrics.as_deref_mut() {
                metrics.line_count += 1;
            }
            trace_events.push(TraceEvent::Line {
                line: trace_line(line, preview_trace),
            });
        }
        PreviewEvent::Prompt(prompt) => {
            run.prompt(
                prompt.line().map(run_line),
                prompt
                    .choices()
                    .iter()
                    .map(|choice| RunChoice {
                        id: choice.id.as_str().to_owned(),
                        text: choice.text.clone(),
                        available: choice.availability.is_available,
                    })
                    .collect(),
            );
            if let Some(metrics) = metrics.as_deref_mut() {
                metrics.prompt_count += 1;
                metrics.choice_count += prompt.choices().len();
                if prompt.line().is_some() {
                    metrics.line_count += 1;
                }
            }
            trace_events.push(TraceEvent::Prompt {
                prompt: trace_prompt(prompt, preview_trace, prompt_cardinality),
            });
        }
        PreviewEvent::ChoiceSelected { prompt, choice_id } => {
            run.select(choice_id.as_str());
            trace_events.push(TraceEvent::ChoiceSelected {
                prompt: TracePromptIdentity {
                    block: prompt.block().as_str().to_owned(),
                    line: prompt.line().map(|line| line.as_str().to_owned()),
                    fixture_keys: fixture_keys(prompt, prompt_cardinality),
                },
                choice: choice_id.as_str().to_owned(),
            });
        }
        PreviewEvent::EffectRequested(effect) => {
            run.effect(
                effect.id.as_str().to_owned(),
                effect_call(effect, messages),
                effect.mode.to_string(),
            );
            if let Some(metrics) = metrics.as_deref_mut() {
                metrics.record_effect(effect.mode);
            }
            trace_events.push(TraceEvent::Effect {
                effect: trace_effect(effect),
            });
        }
        PreviewEvent::EffectAcknowledged { effect_id, .. } => {
            run.acknowledge(effect_id.as_str());
            trace_events.push(TraceEvent::Acknowledgement {
                effect_id: effect_id.as_str().to_owned(),
                result: "completed",
            });
        }
        PreviewEvent::End { deferred_effects } => {
            run.end(
                deferred_effects
                    .iter()
                    .map(|effect| effect_call(effect, messages))
                    .collect(),
            );
            if let Some(metrics) = metrics {
                for effect in deferred_effects {
                    metrics.record_effect(effect.mode);
                }
            }
            trace_events.push(TraceEvent::End {
                deferred_effects: deferred_effects.iter().map(trace_effect).collect(),
            });
        }
        PreviewEvent::ConditionRequested(_)
        | PreviewEvent::ChoiceAccepted { .. }
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

fn run_line(line: &recite_runtime::DialogueLine) -> RunLine {
    RunLine {
        id: line.id.as_str().to_owned(),
        speaker: line
            .speaker
            .as_ref()
            .map(|speaker| speaker.as_str().to_owned()),
        text: line.text.clone(),
    }
}

fn effect_call(effect: &recite_runtime::DialogueEffectRequest, messages: &Messages) -> String {
    messages.format(
        MsgId::RunEffect,
        [
            ("function", UiArg::from(effect.function.clone())),
            (
                "args",
                UiArg::from(super::trace::format_effect_arguments(&effect.args)),
            ),
        ],
    )
}

fn trace_prompt(
    prompt: &PreviewPrompt,
    preview_trace: &PreviewTrace,
    prompt_cardinality: &PromptCardinality,
) -> TracePrompt {
    let identity = prompt.identity();
    TracePrompt {
        identity: TracePromptIdentity {
            block: identity.block().as_str().to_owned(),
            line: identity.line().map(|line| line.as_str().to_owned()),
            fixture_keys: fixture_keys(identity, prompt_cardinality),
        },
        line: prompt.line().map(|line| trace_line(line, preview_trace)),
        choices: prompt
            .choices()
            .iter()
            .map(|choice| trace_choice(choice, preview_trace))
            .collect(),
    }
}

fn fixture_keys(
    identity: &recite_runtime::preview::PreviewPromptIdentity,
    prompt_cardinality: &PromptCardinality,
) -> Vec<String> {
    let mut keys = identity
        .line()
        .map(|line| vec![line.as_str().to_owned()])
        .unwrap_or_default();
    if prompt_cardinality.for_block(identity.block()) == 1 {
        keys.push(identity.block().as_str().to_owned());
    }
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
