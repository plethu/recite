use std::collections::VecDeque;
use std::time::Instant;

use recite_core::CompiledDialogue;
use recite_runtime::{PreviewEvent, PreviewInputs, PreviewOptions, PreviewSession};

use super::condition::{condition_answer, make_inputs_revision};
use super::fixture::RuntimeFixture;
use super::metrics::{RuntimeMetricsCollector, record_session_size};
use super::projection::{RuntimeExecution, project_event};
use super::prompt::{PromptCardinality, PromptIdentity, select_fixture_choice};
use super::trace::{TraceDocument, trace_effect};
use crate::dialogue_locale::DialogueTraversalPreview;
use crate::error::CliError;
use crate::i18n::Messages;

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
    let preview_options = dialogue_preview.map_or_else(PreviewOptions::new, |preview| {
        PreviewOptions::new().with_locale(preview.locale().clone())
    });
    let inputs = make_inputs(dialogue_preview, fixture);
    let mut session = PreviewSession::new(asset, Some(block), preview_options)?;
    let prompt_cardinality = PromptCardinality::new(asset)?;
    let mut metrics = options.metrics.then(RuntimeMetricsCollector::default);
    record_session_size(metrics.as_mut(), session.session())?;
    // Wall-clock duration is intentionally opt-in trace instrumentation; the default trace is
    // deterministic and contains no timing data.
    // Reason: Instant is isolated to opt-in metrics and never enters the deterministic trace
    // contract; replacing it would make the metrics instrumentation less faithful.
    #[allow(
        clippy::disallowed_methods,
        reason = "opt-in wall-clock metrics stay outside the deterministic trace contract"
    )]
    let metrics_started_at = options.metrics.then(Instant::now);
    let mut run_lines = Vec::new();
    let mut trace_events = Vec::new();
    let mut final_deferred_effects = Vec::new();
    let mut choice_in_flight = false;
    let mut deferred_condition_events = Vec::new();
    let mut ended = false;

    while !ended {
        let output = session.step(inputs);
        record_session_size(metrics.as_mut(), session.session())?;
        let mut pending = VecDeque::from(output.events().to_vec());
        while let Some(event) = pending.pop_front() {
            if let PreviewEvent::Error(error) = &event {
                return Err(preview_failure(error.clone()));
            }
            if choice_in_flight && matches!(event, PreviewEvent::ConditionResult { .. }) {
                deferred_condition_events.push(event.clone());
            } else {
                project_event(
                    &event,
                    session.trace(),
                    &prompt_cardinality,
                    &mut run_lines,
                    &mut trace_events,
                    metrics.as_mut(),
                    messages,
                )?;
            }
            match event {
                PreviewEvent::ConditionRequested(request) => {
                    let answer = condition_answer(fixture, &request)?;
                    let output = session.answer(request.id(), answer, inputs);
                    record_session_size(metrics.as_mut(), session.session())?;
                    pending.extend(output.events().iter().cloned());
                }
                PreviewEvent::Prompt(prompt) => {
                    let block_prompt_count =
                        prompt_cardinality.for_block(prompt.identity().block());
                    let identity = PromptIdentity::from_preview(&prompt, block_prompt_count);
                    let choice_id = select_fixture_choice(
                        fixture,
                        &identity,
                        prompt.choices(),
                        block_prompt_count,
                    )?;
                    let output = session.choose(choice_id, inputs);
                    choice_in_flight = true;
                    record_session_size(metrics.as_mut(), session.session())?;
                    pending.extend(output.events().iter().cloned());
                }
                PreviewEvent::EffectRequested(effect)
                    if effect.mode == recite_runtime::DialogueEffectMode::Blocking =>
                {
                    if !fixture.effects.auto_ack_blocking {
                        return Err(CliError::BlockingEffectNeedsAcknowledgement {
                            effect: effect.id.as_str().to_owned(),
                        });
                    }
                    let output = session
                        .acknowledge(effect.id.clone(), recite_runtime::EffectAck::Completed);
                    record_session_size(metrics.as_mut(), session.session())?;
                    pending.extend(output.events().iter().cloned());
                }
                PreviewEvent::End { deferred_effects } => {
                    final_deferred_effects = deferred_effects.iter().map(trace_effect).collect();
                    ended = true;
                }
                PreviewEvent::ChoiceSelected { .. } => {
                    choice_in_flight = false;
                    let deferred = std::mem::take(&mut deferred_condition_events);
                    for condition in deferred {
                        project_event(
                            &condition,
                            session.trace(),
                            &prompt_cardinality,
                            &mut run_lines,
                            &mut trace_events,
                            metrics.as_mut(),
                            messages,
                        )?;
                    }
                }
                PreviewEvent::ConditionResult { .. }
                | PreviewEvent::Line(_)
                | PreviewEvent::EffectRequested(_)
                | PreviewEvent::EffectAcknowledged { .. }
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
        }
    }

    let metrics = metrics.map(|metrics| {
        metrics.finish(
            trace_events.len(),
            session.trace().localized_lookups().count(),
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
            session
                .trace()
                .locale()
                .map(|locale| locale.as_str().to_owned()),
            dialogue_locale_fallbacks,
            trace_events,
            final_deferred_effects,
            metrics,
        ),
    })
}

fn preview_failure(error: recite_runtime::PreviewError) -> CliError {
    match error {
        recite_runtime::PreviewError::Runtime(
            recite_runtime::DialogueError::MalformedCompiledAsset { reason },
        ) => CliError::MalformedCompiledAsset { reason },
        recite_runtime::PreviewError::Runtime(error) => CliError::Runtime(error),
        recite_runtime::PreviewError::AssetRevisionFailed { reason } => {
            CliError::MalformedCompiledAsset { reason }
        }
        error => CliError::Preview(error),
    }
}

fn make_inputs<'a>(
    preview: Option<DialogueTraversalPreview<'a>>,
    fixture: &'a RuntimeFixture,
) -> PreviewInputs<'a> {
    let inputs = PreviewInputs::new()
        .with_interpolation_values(fixture.interpolation_values())
        .with_revision(make_inputs_revision());
    preview.map_or(inputs, |preview| {
        inputs.with_locale_provider(preview.provider())
    })
}
