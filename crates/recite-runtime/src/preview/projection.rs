use std::sync::Arc;

use super::PreviewSession;
use super::model::{
    PreviewError, PreviewEvent, PreviewOutput, PreviewRestartRequirement, PreviewStatus,
};

impl<'asset> PreviewSession<'asset> {
    pub(super) fn append_events(&mut self, events: Vec<PreviewEvent>) -> PreviewOutput {
        for event in &events {
            self.apply_event(event);
            self.trace.push(event.clone());
            self.transcript.push(event);
        }
        self.refresh_state_projection();
        PreviewOutput::new(events, self.state.clone())
    }

    fn apply_event(&mut self, event: &PreviewEvent) {
        match event {
            PreviewEvent::ConditionRequested(_) | PreviewEvent::ConditionResult { .. } => {}
            PreviewEvent::Line(_) => self.state.status = PreviewStatus::Ready,
            PreviewEvent::Prompt(prompt) => {
                self.state.status = PreviewStatus::WaitingForChoice {
                    prompt: Box::new(prompt.clone()),
                };
            }
            PreviewEvent::ChoiceAccepted { .. } => {}
            PreviewEvent::ChoiceSelected { .. } => self.state.status = PreviewStatus::Ready,
            PreviewEvent::EffectRequested(effect) => {
                self.state.status = if effect.mode == crate::DialogueEffectMode::Blocking {
                    PreviewStatus::WaitingForEffect {
                        effect: effect.clone(),
                    }
                } else {
                    PreviewStatus::Ready
                };
            }
            PreviewEvent::DeferredEffectScheduled(_) => {}
            PreviewEvent::EffectAcknowledged { .. } => self.state.status = PreviewStatus::Ready,
            PreviewEvent::End { .. } => self.state.status = PreviewStatus::Ended,
            PreviewEvent::Restarted { .. } | PreviewEvent::Restored => {}
            PreviewEvent::RestartRequired {
                active_asset,
                replacement_asset,
                active_revision,
                replacement_revision,
            } => {
                self.state.restart_required = Some(PreviewRestartRequirement::new(
                    active_asset.clone(),
                    replacement_asset.clone(),
                    active_revision.clone(),
                    replacement_revision.clone(),
                ));
            }
            PreviewEvent::Error(_) => {}
        }
    }

    fn refresh_state_projection(&mut self) {
        self.state.block = self.current_block_id_opt();
        self.state.locale = self.session.locale().cloned();
        // Choice history only appends during traversal. Restart and restore
        // install a new projection; unchanged rows can stay shared.
        if self.state.selected_choice_history.len() != self.session.selected_choice_history().len()
        {
            self.state.selected_choice_history = self.session.selected_choice_history().into();
        }
        // Deferred mutations copy shared histories, keeping earlier outputs
        // immutable. Restore also reuses the validated session allocation.
        self.state.deferred_effects = Arc::clone(&self.session.deferred_effects);
    }

    pub(super) fn error(&mut self, error: PreviewError) -> PreviewOutput {
        self.error_with_prefix(None, error)
    }

    pub(super) fn error_with_prefix(
        &mut self,
        prefix: Option<Vec<PreviewEvent>>,
        error: PreviewError,
    ) -> PreviewOutput {
        let mut events = prefix.unwrap_or_default();
        events.push(PreviewEvent::Error(error));
        self.append_events(events)
    }
}

pub(super) fn new_deferred_events(
    previous_len: usize,
    trial: &crate::DialogueSession,
) -> Vec<PreviewEvent> {
    trial
        .deferred_effects()
        .get(previous_len..)
        .unwrap_or_default()
        .iter()
        .cloned()
        .map(PreviewEvent::DeferredEffectScheduled)
        .collect()
}
