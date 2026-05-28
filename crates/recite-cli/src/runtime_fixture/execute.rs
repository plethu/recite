use std::cell::RefCell;
use std::collections::BTreeMap;

use recite_core::CompiledDialogue;
use recite_runtime::{
    ConditionEvaluationError, ConditionExpectedType, ConditionQuery, ConditionValue,
    DialogueContext, DialogueEffectMode, DialogueEvent, EffectAck, acknowledge_effect,
    choose as runtime_choose, next as runtime_next, start_scene,
};

use super::fixture::{FixtureConditionValue, RuntimeFixture};
use super::prompt::{
    PromptCatalog, select_fixture_choice, trace_prompt, trace_prompt_identity,
    write_prompt_run_lines,
};
use super::trace::{
    TraceCondition, TraceConditionValue, TraceDocument, TraceEffect, TraceEvent,
    condition_query_text, format_effect_arguments, trace_condition_argument, trace_effect,
    trace_line,
};
use crate::error::CliError;

pub(crate) struct RuntimeExecution {
    pub(crate) run_lines: Vec<String>,
    pub(crate) trace: TraceDocument,
}

pub(crate) fn execute_runtime_fixture(
    asset: &CompiledDialogue,
    block: &str,
    fixture: &RuntimeFixture,
) -> Result<RuntimeExecution, CliError> {
    let prompt_catalog = PromptCatalog::new(asset)?;
    let context = FixtureContext::new(&fixture.conditions);
    let mut session = start_scene(asset, Some(block))?;
    let mut trace_events = Vec::new();
    let mut run_lines = Vec::new();
    let mut pending_event = None;
    let final_deferred_effects: Vec<TraceEffect>;

    loop {
        let event = match pending_event.take() {
            Some(event) => event,
            None => {
                let event = runtime_next(asset, &mut session, &context)?;
                record_conditions(&context, &mut run_lines, &mut trace_events);
                event
            }
        };

        match event {
            DialogueEvent::Line(line) => {
                run_lines.push(format!("line {}: {}", line.id.as_str(), line.text));
                trace_events.push(TraceEvent::Line {
                    line: trace_line(&line),
                });
            }
            DialogueEvent::Prompt { line, choices } => {
                let prompt = prompt_catalog.identify(line.as_ref(), &choices)?;
                write_prompt_run_lines(&mut run_lines, &prompt, line.as_ref(), &choices);
                trace_events.push(TraceEvent::Prompt {
                    prompt: trace_prompt(&prompt, line.as_ref(), &choices),
                });

                let choice_id = select_fixture_choice(fixture, &prompt, &choices)?;
                run_lines.push(format!("selected choice {}", choice_id.as_str()));
                trace_events.push(TraceEvent::ChoiceSelected {
                    prompt: trace_prompt_identity(&prompt),
                    choice: choice_id.as_str().to_owned(),
                });

                let event = runtime_choose(asset, &mut session, choice_id, &context)?;
                record_conditions(&context, &mut run_lines, &mut trace_events);
                pending_event = Some(event);
            }
            DialogueEvent::Effect(effect) => {
                run_lines.push(format!(
                    "effect {} {} {}",
                    effect.mode,
                    effect.function,
                    format_effect_arguments(&effect.args)
                ));
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
                    run_lines.push(format!(
                        "acknowledged effect {} completed",
                        effect.id.as_str()
                    ));
                    trace_events.push(TraceEvent::Acknowledgement {
                        effect_id: effect.id.as_str().to_owned(),
                        result: "completed",
                    });
                }
            }
            DialogueEvent::End { deferred_effects } => {
                run_lines.push("end".to_owned());
                if !deferred_effects.is_empty() {
                    run_lines.push("deferred effects:".to_owned());
                    for effect in &deferred_effects {
                        run_lines.push(format!(
                            "  {} {}",
                            effect.function,
                            format_effect_arguments(&effect.args)
                        ));
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

    Ok(RuntimeExecution {
        run_lines,
        trace: TraceDocument::new(
            asset.header.asset_id.as_str().to_owned(),
            block.to_owned(),
            trace_events,
            final_deferred_effects,
        ),
    })
}

fn record_conditions(
    context: &FixtureContext<'_>,
    run_lines: &mut Vec<String>,
    trace_events: &mut Vec<TraceEvent>,
) {
    for condition in context.take_records() {
        run_lines.push(format!(
            "condition {} = {}",
            condition.query, condition.result
        ));
        trace_events.push(TraceEvent::Condition { condition });
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
