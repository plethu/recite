use recite_core::CompiledDialogue;
use recite_runtime::{
    DialogueEffectMode, DialogueEvent, DialogueSession, EffectAck, acknowledge_effect,
};

use crate::dialogue_locale::{DialogueTraversal, DialogueTraversalPreview};
use crate::error::CliError;

pub(super) use super::driver_api::{ChoiceSelection, PlayUiAdapter};
use super::driver_context::InteractiveContext;

pub(super) struct PlayDriver<'a> {
    asset: &'a CompiledDialogue,
    block: &'a str,
    dialogue_preview: Option<DialogueTraversalPreview<'a>>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum DeferredQueueStatus {
    Scheduled,
    Ready,
}

impl<'a> PlayDriver<'a> {
    pub(super) fn new(asset: &'a CompiledDialogue, block: &'a str) -> Self {
        Self {
            asset,
            block,
            dialogue_preview: None,
        }
    }

    pub(super) fn with_dialogue_preview(mut self, preview: DialogueTraversalPreview<'a>) -> Self {
        self.dialogue_preview = Some(preview);
        self
    }

    pub(super) fn run<U: PlayUiAdapter>(self, ui: &mut U) -> Result<(), CliError> {
        ui.start(self.asset, self.block)?;
        let context = InteractiveContext::new(ui);
        let traversal = DialogueTraversal::new(self.asset, self.dialogue_preview);
        let mut session = traversal.start(Some(self.block))?;
        let mut pending_event = None;
        let mut deferred_effect_count = 0;

        loop {
            let event = match pending_event.take() {
                Some(event) => event,
                None => match traversal.next(&mut session, &context) {
                    Ok(event) => {
                        notify_scheduled_deferred_queue(
                            &context,
                            &session,
                            &mut deferred_effect_count,
                        )?;
                        event
                    }
                    Err(error) => return Err(context.resolve_runtime_error(error)),
                },
            };
            match event {
                DialogueEvent::Line(line) => context.line(&line)?,
                DialogueEvent::Prompt { line, choices } => {
                    let choice_id = context.choice(line.as_ref(), &choices)?;
                    context.selected_choice(&choice_id)?;
                    let event = traversal
                        .choose(&mut session, choice_id.clone(), &context)
                        .map_err(|error| context.resolve_runtime_error(error))?;
                    notify_scheduled_deferred_queue(
                        &context,
                        &session,
                        &mut deferred_effect_count,
                    )?;
                    pending_event = Some(event);
                }
                DialogueEvent::Effect(effect) => {
                    context.effect(&effect)?;
                    if effect.mode == DialogueEffectMode::Blocking {
                        context.acknowledge(&effect)?;
                        acknowledge_effect(&mut session, effect.id.clone(), EffectAck::Completed)?;
                    }
                }
                DialogueEvent::End { deferred_effects } => {
                    context.deferred_queue(&deferred_effects, DeferredQueueStatus::Ready)?;
                    context.end(&deferred_effects)?;
                    break;
                }
            }
        }
        Ok(())
    }
}

fn notify_scheduled_deferred_queue<U: PlayUiAdapter>(
    context: &InteractiveContext<'_, U>,
    session: &DialogueSession,
    deferred_effect_count: &mut usize,
) -> Result<(), CliError> {
    let deferred_effects = session.deferred_effects();
    if deferred_effects.len() != *deferred_effect_count {
        context.deferred_queue(deferred_effects, DeferredQueueStatus::Scheduled)?;
        *deferred_effect_count = deferred_effects.len();
    }
    Ok(())
}

#[cfg(test)]
mod tests;
