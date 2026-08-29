use std::cell::RefCell;
use std::collections::BTreeMap;
use std::time::Instant;

use recite_core::CompiledDialogue;
use recite_runtime::{
    ConditionEvaluationError, ConditionExpectedType, ConditionQuery, ConditionValue,
    DialogueContext, DialogueEffectMode, DialogueEvent, DialogueTrace, EffectAck,
    acknowledge_effect,
};
use recite_ui::UiArg;

use super::fixture::{FixtureConditionValue, RuntimeFixture};
use super::metrics::{CountingLocaleProvider, RuntimeMetricsCollector, record_session_size};
use super::prompt::{
    PromptCatalog, select_fixture_choice, trace_prompt, trace_prompt_identity,
    write_prompt_run_lines,
};
use super::trace::{
    TraceCondition, TraceConditionValue, TraceDocument, TraceEffect, TraceEvent,
    condition_query_text, format_effect_arguments, trace_condition_argument, trace_effect,
    trace_line,
};
use crate::dialogue_locale::{DialogueTraversal, DialogueTraversalPreview};
use crate::error::CliError;
use crate::i18n::{Messages, MsgId};

pub(crate) struct RuntimeExecution {
    pub(crate) run_lines: Vec<String>,
    pub(crate) trace: TraceDocument,
}

#[derive(Clone, Copy, Debug, Default)]
pub(crate) struct RuntimeFixtureOptions {
    pub(crate) metrics: bool,
}

pub(crate) fn execute_runtime_fixture(
    asset: &CompiledDialogue,
    block: &str,
    fixture: &RuntimeFixture,
    dialogue_preview: Option<DialogueTraversalPreview<'_>>,
    dialogue_locale_fallbacks: Option<Vec<String>>,
    options: RuntimeFixtureOptions,
    messages: &Messages,
) -> Result<RuntimeExecution, CliError> {
    let prompt_catalog = PromptCatalog::new(asset)?;
    let context = FixtureContext::new(&fixture.conditions);
    let counting_provider = dialogue_preview
        .filter(|_| options.metrics)
        .map(|preview| CountingLocaleProvider::new(preview.provider()));
    let traversal_preview = match (dialogue_preview, counting_provider.as_ref()) {
        (Some(preview), Some(provider)) => {
            Some(DialogueTraversalPreview::new(preview.locale(), provider))
        }
        (preview, None) => preview,
        (None, Some(_)) => None,
    };
    let dialogue_trace = DialogueTrace::new();
    let traversal = DialogueTraversal::new(asset, traversal_preview)
        .with_values(fixture.interpolation_values())
        .with_trace(&dialogue_trace);
    // Wall-clock duration is intentionally opt-in trace instrumentation; the
    // default deterministic trace output does not include it.
    #[allow(clippy::disallowed_methods)]
    let metrics_started_at = options.metrics.then(Instant::now);
    let mut metrics = options.metrics.then(RuntimeMetricsCollector::default);
    let mut session = traversal.start(Some(block))?;
    record_session_size(metrics.as_mut(), &session)?;
    let mut trace_events = Vec::new();
    let mut run_lines = Vec::new();
    let mut pending_event = None;
    let final_deferred_effects: Vec<TraceEffect>;

    loop {
        let event = match pending_event.take() {
            Some(event) => event,
            None => {
                let event = traversal.next(&mut session, &context)?;
                record_session_size(metrics.as_mut(), &session)?;
                record_conditions(
                    &context,
                    &mut run_lines,
                    &mut trace_events,
                    metrics.as_mut(),
                    messages,
                );
                event
            }
        };

        match event {
            DialogueEvent::Line(line) => {
                run_lines.push(messages.format(
                    MsgId::PlayLine,
                    [
                        ("id", UiArg::from(line.id.as_str())),
                        ("text", UiArg::from(line.text.as_str())),
                    ],
                ));
                if let Some(metrics) = metrics.as_mut() {
                    metrics.line_count += 1;
                }
                trace_events.push(TraceEvent::Line {
                    line: trace_line(&line, &dialogue_trace),
                });
            }
            DialogueEvent::Prompt { line, choices } => {
                let prompt = prompt_catalog.identify(line.as_ref(), &choices)?;
                write_prompt_run_lines(&mut run_lines, &prompt, line.as_ref(), &choices, messages);
                if let Some(metrics) = metrics.as_mut() {
                    metrics.prompt_count += 1;
                    metrics.choice_count += choices.len();
                    if line.is_some() {
                        metrics.line_count += 1;
                    }
                }
                trace_events.push(TraceEvent::Prompt {
                    prompt: trace_prompt(&prompt, line.as_ref(), &choices, &dialogue_trace),
                });

                let choice_id = select_fixture_choice(fixture, &prompt, &choices)?;
                run_lines.push(messages.format(
                    MsgId::PlaySelectedChoice,
                    [("id", UiArg::from(choice_id.as_str()))],
                ));
                trace_events.push(TraceEvent::ChoiceSelected {
                    prompt: trace_prompt_identity(&prompt),
                    choice: choice_id.as_str().to_owned(),
                });

                let event = traversal.choose(&mut session, choice_id, &context)?;
                record_session_size(metrics.as_mut(), &session)?;
                record_conditions(
                    &context,
                    &mut run_lines,
                    &mut trace_events,
                    metrics.as_mut(),
                    messages,
                );
                pending_event = Some(event);
            }
            DialogueEvent::Effect(effect) => {
                run_lines.push(messages.format(
                    MsgId::RunEffect,
                    [
                        ("mode", UiArg::from(effect.mode.to_string())),
                        ("function", UiArg::from(effect.function.clone())),
                        ("args", UiArg::from(format_effect_arguments(&effect.args))),
                    ],
                ));
                if let Some(metrics) = metrics.as_mut() {
                    metrics.record_effect(effect.mode);
                }
                trace_events.push(TraceEvent::Effect {
                    effect: trace_effect(&effect),
                });

                if effect.mode == DialogueEffectMode::Blocking {
                    if !fixture.effects.auto_ack_blocking {
                        return Err(CliError::BlockingEffectNeedsAcknowledgement {
                            effect: effect.id.as_str().to_owned(),
                        });
                    }

                    acknowledge_effect(&mut session, effect.id.clone(), EffectAck::Completed)?;
                    record_session_size(metrics.as_mut(), &session)?;
                    run_lines.push(messages.format(
                        MsgId::PlayAckCompleted,
                        [("id", UiArg::from(effect.id.as_str()))],
                    ));
                    trace_events.push(TraceEvent::Acknowledgement {
                        effect_id: effect.id.as_str().to_owned(),
                        result: "completed",
                    });
                }
            }
            DialogueEvent::End { deferred_effects } => {
                run_lines.push(messages.text(MsgId::PlayEnd));
                if !deferred_effects.is_empty() {
                    run_lines.push(messages.text(MsgId::PlayDeferredEffects));
                    for effect in &deferred_effects {
                        run_lines.push(messages.format(
                            MsgId::PlayDeferredEffectRow,
                            [
                                ("function", UiArg::from(effect.function.clone())),
                                ("args", UiArg::from(format_effect_arguments(&effect.args))),
                            ],
                        ));
                    }
                }

                if let Some(metrics) = metrics.as_mut() {
                    for effect in &deferred_effects {
                        metrics.record_effect(effect.mode);
                    }
                }
                final_deferred_effects = deferred_effects.iter().map(trace_effect).collect();
                trace_events.push(TraceEvent::End {
                    deferred_effects: final_deferred_effects.clone(),
                });
                break;
            }
        }
    }

    let metrics = metrics.map(|metrics| {
        metrics.finish(
            trace_events.len(),
            counting_provider
                .as_ref()
                .map(CountingLocaleProvider::lookup_count)
                .unwrap_or(0),
            metrics_started_at
                .map(|started_at| started_at.elapsed().as_nanos())
                .unwrap_or(0),
        )
    });

    Ok(RuntimeExecution {
        run_lines,
        trace: TraceDocument::new(
            asset.header.asset_id.as_str().to_owned(),
            block.to_owned(),
            dialogue_preview.map(|preview| preview.locale().as_str().to_owned()),
            dialogue_locale_fallbacks,
            trace_events,
            final_deferred_effects,
            metrics,
        ),
    })
}

fn record_conditions(
    context: &FixtureContext<'_>,
    run_lines: &mut Vec<String>,
    trace_events: &mut Vec<TraceEvent>,
    metrics: Option<&mut RuntimeMetricsCollector>,
    messages: &Messages,
) {
    let mut metrics = metrics;
    for condition in context.take_records() {
        run_lines.push(messages.format(
            MsgId::PlayConditionResult,
            [
                ("query", UiArg::from(condition.query.clone())),
                ("result", UiArg::from(condition.result.to_string())),
            ],
        ));
        trace_events.push(TraceEvent::Condition { condition });
        if let Some(metrics) = metrics.as_deref_mut() {
            metrics.condition_evaluation_count += 1;
        }
    }
}

struct FixtureContext<'a> {
    conditions: &'a BTreeMap<String, FixtureConditionValue>,
    records: RefCell<Vec<TraceCondition>>,
}

impl<'a> FixtureContext<'a> {
    fn new(conditions: &'a BTreeMap<String, FixtureConditionValue>) -> Self {
        Self {
            conditions,
            records: RefCell::new(Vec::new()),
        }
    }

    fn take_records(&self) -> Vec<TraceCondition> {
        self.records.take()
    }
}

impl DialogueContext for FixtureContext<'_> {
    fn evaluate_condition(
        &self,
        query: ConditionQuery<'_>,
    ) -> Result<ConditionValue, ConditionEvaluationError> {
        let arguments = query
            .arguments()
            .into_iter()
            .map(trace_condition_argument)
            .collect::<Vec<_>>();
        let query_text = condition_query_text(query.function(), &arguments);
        let Some(result) = self.conditions.get(&query_text) else {
            return Err(ConditionEvaluationError::new(format!(
                "fixture is missing condition `{query_text}`"
            )));
        };
        let result = match (query.expected_type(), result) {
            (ConditionExpectedType::Bool, FixtureConditionValue::Bool(value)) => {
                ConditionValue::Bool(*value)
            }
            (ConditionExpectedType::Enum, FixtureConditionValue::Enum { r#enum }) => {
                ConditionValue::EnumVariant(r#enum.clone())
            }
            (ConditionExpectedType::Bool, FixtureConditionValue::Enum { .. }) => {
                ConditionValue::EnumVariant("<fixture enum>".to_owned())
            }
            (ConditionExpectedType::Enum, FixtureConditionValue::Bool(value)) => {
                ConditionValue::Bool(*value)
            }
        };

        self.records.borrow_mut().push(TraceCondition {
            query: query_text,
            function: query.function().to_owned(),
            arguments,
            result: trace_condition_value(&result),
        });
        Ok(result)
    }
}

fn trace_condition_value(value: &ConditionValue) -> TraceConditionValue {
    match value {
        ConditionValue::Bool(value) => TraceConditionValue::Bool(*value),
        ConditionValue::EnumVariant(value) => TraceConditionValue::EnumVariant {
            r#enum: value.clone(),
        },
    }
}
