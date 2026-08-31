use std::collections::VecDeque;

use recite_core::{ChoiceId, CompiledDialogue};
use recite_runtime::{
    ConditionAnswer, DialogueEffectMode, EffectAck, PreviewCommand, PreviewConditionRequest,
    PreviewConditionResult, PreviewError, PreviewEvent, PreviewInputRevision, PreviewInputs,
    PreviewOptions, PreviewPrompt, PreviewSession,
};

use crate::dialogue_locale::DialogueTraversalPreview;
use crate::error::CliError;

/// Presentation-neutral event loop for interactive preview consumers.
///
/// The presentation supplies typed commands at runtime boundaries. It never owns a
/// `DialogueSession`, condition cache, or effect queue; those remain in `PreviewSession`.
pub(super) fn run_preview<U: PreviewPlayUi>(
    asset: &CompiledDialogue,
    block: &str,
    dialogue_preview: Option<DialogueTraversalPreview<'_>>,
    ui: &mut U,
) -> Result<(), CliError> {
    let options = dialogue_preview.map_or_else(PreviewOptions::new, |preview| {
        PreviewOptions::new().with_locale(preview.locale().clone())
    });
    let inputs = preview_inputs(dialogue_preview);
    ui.start(asset, block)?;
    let mut session =
        PreviewSession::new(asset, Some(block), options).map_err(CliError::Runtime)?;
    let mut pending = VecDeque::new();
    let mut announced_choice = None;

    loop {
        if pending.is_empty() {
            pending.extend(
                session
                    .dispatch(PreviewCommand::Advance, inputs)
                    .events()
                    .iter()
                    .cloned(),
            );
        }
        let Some(event) = pending.pop_front() else {
            return Err(CliError::Preview(PreviewError::Runtime(
                recite_runtime::DialogueError::MalformedCompiledAsset {
                    reason: "preview produced no event".to_owned(),
                },
            )));
        };

        match event {
            PreviewEvent::ConditionRequested(request) => {
                let answer = loop {
                    match ui.condition(&request) {
                        Ok(answer) => break answer,
                        Err(CliError::PlayInvalidInput(message)) => {
                            ui.invalid_input(message)?;
                        }
                        Err(error) => return Err(error),
                    }
                };
                let output = session.dispatch(
                    PreviewCommand::Answer {
                        request_id: request.id(),
                        answer,
                    },
                    inputs,
                );
                enqueue_events(&mut pending, output.events(), &mut announced_choice);
            }
            PreviewEvent::ConditionResult { request, result } => {
                ui.condition_result(&request, &result)?;
            }
            PreviewEvent::Line(line) => ui.line(&line)?,
            PreviewEvent::Prompt(prompt) => {
                let choice_id = loop {
                    match ui.choice(&prompt) {
                        Ok(choice_id) => break choice_id,
                        Err(CliError::PlayInvalidInput(message)) => {
                            ui.invalid_input(message)?;
                        }
                        Err(error) => return Err(error),
                    }
                };
                let output = session.dispatch(
                    PreviewCommand::Choose {
                        choice_id: choice_id.clone(),
                    },
                    inputs,
                );
                if output
                    .events()
                    .iter()
                    .any(|event| matches!(event, PreviewEvent::ConditionRequested(_)))
                    && !output
                        .events()
                        .iter()
                        .any(|event| matches!(event, PreviewEvent::Error(_)))
                    && !output
                        .events()
                        .iter()
                        .any(|event| matches!(event, PreviewEvent::ChoiceSelected { .. }))
                {
                    // Choice validation succeeded at the preview boundary. Present this
                    // accepted transition before asking for a follow-up branch condition;
                    // the eventual runtime ChoiceSelected event is consumed below so the
                    // presenter sees the transition exactly once.
                    ui.selected_choice(&choice_id)?;
                    announced_choice = Some(choice_id);
                }
                enqueue_events(&mut pending, output.events(), &mut announced_choice);
            }
            PreviewEvent::ChoiceSelected { choice_id, .. } => ui.selected_choice(&choice_id)?,
            PreviewEvent::EffectRequested(effect) => {
                let blocking = effect.mode == DialogueEffectMode::Blocking;
                ui.effect(&effect)?;
                if blocking {
                    ui.acknowledge(&effect)?;
                    let output = session.dispatch(
                        PreviewCommand::Acknowledge {
                            effect_id: effect.id.clone(),
                            ack: EffectAck::Completed,
                        },
                        PreviewInputs::default(),
                    );
                    pending.extend(output.events().iter().cloned());
                }
            }
            PreviewEvent::EffectAcknowledged { effect_id, .. } => {
                ui.acknowledged(&effect_id)?;
            }
            PreviewEvent::End { deferred_effects } => {
                ui.end(&deferred_effects)?;
                return Ok(());
            }
            PreviewEvent::DeferredEffectScheduled(effect) => {
                ui.deferred_effect_scheduled(&effect)?;
            }
            PreviewEvent::Restarted { .. } | PreviewEvent::Restored => {}
            PreviewEvent::Error(error) => return Err(preview_error(error)),
            _ => {
                return Err(CliError::Preview(PreviewError::Runtime(
                    recite_runtime::DialogueError::MalformedCompiledAsset {
                        reason: "preview emitted an unsupported structured event".to_owned(),
                    },
                )));
            }
        }
    }
}

/// Presents accepted choices before replayed branch conditions without changing runtime trace.
fn enqueue_events(
    pending: &mut VecDeque<PreviewEvent>,
    events: &[PreviewEvent],
    announced_choice: &mut Option<ChoiceId>,
) {
    let mut events = events
        .iter()
        .filter(|event| {
            let PreviewEvent::ChoiceSelected { choice_id, .. } = event else {
                return true;
            };
            if announced_choice.as_ref() == Some(choice_id) {
                *announced_choice = None;
                false
            } else {
                true
            }
        })
        .cloned()
        .collect::<Vec<_>>();
    if announced_choice.is_some() {
        pending.extend(events);
        return;
    }
    let Some(choice_index) = events
        .iter()
        .position(|event| matches!(event, PreviewEvent::ChoiceSelected { .. }))
    else {
        pending.extend(events);
        return;
    };

    pending.push_back(events.remove(choice_index));
    pending.extend(events);
}

fn preview_inputs(preview: Option<DialogueTraversalPreview<'_>>) -> PreviewInputs<'_> {
    // Play owns one immutable invocation: catalog contents and any future host
    // inputs are held behind this revision for the duration of the session.
    // PreviewSession enforces the revision when a condition answer replays a
    // suspended operation; it cannot detect mutation hidden behind a reused
    // provider reference, which remains the caller's honesty boundary.
    let inputs = PreviewInputs::new().with_revision(PreviewInputRevision::new(0));
    preview.map_or(inputs, |preview| {
        inputs.with_locale_provider(preview.provider())
    })
}

fn preview_error(error: PreviewError) -> CliError {
    match error {
        PreviewError::Runtime(error) => CliError::Runtime(error),
        PreviewError::ConditionResultTypeMismatch {
            function,
            expected,
            actual,
        } => CliError::Runtime(recite_runtime::DialogueError::ConditionResultTypeMismatch {
            function,
            expected,
            actual,
        }),
        error => CliError::Preview(error),
    }
}

pub(super) trait PreviewPlayUi {
    fn start(&mut self, asset: &CompiledDialogue, block: &str) -> Result<(), CliError>;
    fn line(&mut self, line: &recite_runtime::DialogueLine) -> Result<(), CliError>;
    fn choice(&mut self, prompt: &PreviewPrompt) -> Result<ChoiceId, CliError>;
    fn selected_choice(&mut self, choice_id: &ChoiceId) -> Result<(), CliError>;
    fn condition(&mut self, request: &PreviewConditionRequest)
    -> Result<ConditionAnswer, CliError>;
    fn condition_result(
        &mut self,
        request: &PreviewConditionRequest,
        result: &PreviewConditionResult,
    ) -> Result<(), CliError>;
    fn effect(&mut self, effect: &recite_runtime::DialogueEffectRequest) -> Result<(), CliError>;
    fn acknowledge(
        &mut self,
        effect: &recite_runtime::DialogueEffectRequest,
    ) -> Result<(), CliError>;
    fn acknowledged(&mut self, effect_id: &recite_core::EffectId) -> Result<(), CliError>;
    fn deferred_effect_scheduled(
        &mut self,
        _effect: &recite_runtime::DialogueEffectRequest,
    ) -> Result<(), CliError> {
        Ok(())
    }
    fn invalid_input(&mut self, message: String) -> Result<(), CliError>;
    fn end(
        &mut self,
        deferred_effects: &[recite_runtime::DialogueEffectRequest],
    ) -> Result<(), CliError>;
}
