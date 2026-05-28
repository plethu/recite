use std::cell::RefCell;

use recite_core::{ChoiceId, CompiledDialogue};
use recite_runtime::{
    ConditionEvaluationError, ConditionQuery, ConditionValue, DialogueChoice, DialogueContext,
    DialogueEffectMode, DialogueEffectRequest, DialogueEvent, DialogueLine, DialogueSession,
    EffectAck, acknowledge_effect, choose as runtime_choose, next as runtime_next, start_scene,
};

use crate::error::CliError;
use crate::i18n::{Messages, MsgId};

pub(super) struct PlayDriver<'a> {
    asset: &'a CompiledDialogue,
    block: &'a str,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum DeferredQueueStatus {
    Scheduled,
    Ready,
}

impl<'a> PlayDriver<'a> {
    pub(super) fn new(asset: &'a CompiledDialogue, block: &'a str) -> Self {
        Self { asset, block }
    }

    pub(super) fn run<U: PlayUiAdapter>(self, ui: &mut U) -> Result<(), CliError> {
        ui.start(self.asset, self.block)?;
        let context = InteractiveContext::new(ui);
        let mut session = start_scene(self.asset, Some(self.block))?;
        let mut pending_event = None;
        let mut deferred_effect_count = 0;

        loop {
            let event = match pending_event.take() {
                Some(event) => event,
                None => match runtime_next(self.asset, &mut session, &context) {
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
                    let event = runtime_choose(self.asset, &mut session, choice_id, &context)
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

struct InteractiveContext<'a, U> {
    ui: RefCell<&'a mut U>,
    interrupted: RefCell<bool>,
    ui_error: RefCell<Option<CliError>>,
}

impl<'a, U> InteractiveContext<'a, U> {
    fn new(ui: &'a mut U) -> Self {
        Self {
            ui: RefCell::new(ui),
            interrupted: RefCell::new(false),
            ui_error: RefCell::new(None),
        }
    }
}

impl<U: PlayUiAdapter> InteractiveContext<'_, U> {
    fn line(&self, line: &DialogueLine) -> Result<(), CliError> {
        self.ui.borrow_mut().line(line)
    }

    fn choice(
        &self,
        line: Option<&DialogueLine>,
        choices: &[DialogueChoice],
    ) -> Result<ChoiceId, CliError> {
        loop {
            let choice_result = {
                let mut ui = self.ui.borrow_mut();
                ui.choice(line, choices)
            };
            let selection = match choice_result {
                Ok(selection) => selection,
                Err(CliError::PlayInvalidInput(message)) => {
                    self.ui.borrow_mut().invalid_input(message)?;
                    continue;
                }
                Err(error) => return Err(error),
            };
            match selection {
                ChoiceSelection::Index(index) => {
                    let numeric_id = index.to_string();
                    if let Some(choice) = choices
                        .iter()
                        .find(|choice| choice.id.as_str() == numeric_id)
                    {
                        if choice.is_available {
                            return Ok(choice.id.clone());
                        }
                        let message = unavailable_choice_message(&self.ui, choice);
                        self.ui.borrow_mut().invalid_input(message)?;
                        continue;
                    }
                    if index == 0 || index > choices.len() {
                        let message = self.ui.borrow().message(
                            MsgId::PlayErrorChoiceIndexOutOfRange,
                            [
                                ("index", index.to_string()),
                                ("count", choices.len().to_string()),
                            ],
                        );
                        self.ui.borrow_mut().invalid_input(message)?;
                        continue;
                    }
                    let choice = &choices[index - 1];
                    if choice.is_available {
                        return Ok(choice.id.clone());
                    }
                    let message = unavailable_choice_message(&self.ui, choice);
                    self.ui.borrow_mut().invalid_input(message)?;
                }
                ChoiceSelection::Id(id) => {
                    let choice_id = match ChoiceId::new(id.clone()) {
                        Ok(choice_id) => choice_id,
                        Err(error) => {
                            let message = self.ui.borrow().message(
                                MsgId::PlayErrorChoiceIdInvalid,
                                [("id", id), ("error", error.to_string())],
                            );
                            self.ui.borrow_mut().invalid_input(message)?;
                            continue;
                        }
                    };
                    if let Some(choice) = choices.iter().find(|choice| choice.id == choice_id) {
                        if choice.is_available {
                            return Ok(choice_id);
                        }
                        let message = unavailable_choice_message(&self.ui, choice);
                        self.ui.borrow_mut().invalid_input(message)?;
                    } else {
                        let message = self
                            .ui
                            .borrow()
                            .message(MsgId::PlayErrorChoiceIdUnavailable, [("id", id)]);
                        self.ui.borrow_mut().invalid_input(message)?;
                    }
                }
            }
        }
    }

    fn selected_choice(&self, choice_id: &ChoiceId) -> Result<(), CliError> {
        self.ui.borrow_mut().selected_choice(choice_id)
    }

    fn effect(&self, effect: &DialogueEffectRequest) -> Result<(), CliError> {
        self.ui.borrow_mut().effect(effect)
    }

    fn acknowledge(&self, effect: &DialogueEffectRequest) -> Result<(), CliError> {
        self.ui.borrow_mut().acknowledge(effect)
    }

    fn deferred_queue(
        &self,
        effects: &[DialogueEffectRequest],
        status: DeferredQueueStatus,
    ) -> Result<(), CliError> {
        self.ui.borrow_mut().deferred_queue(effects, status)
    }

    fn end(&self, deferred_effects: &[DialogueEffectRequest]) -> Result<(), CliError> {
        self.ui.borrow_mut().end(deferred_effects)
    }

    fn mark_interrupted(&self) {
        *self.interrupted.borrow_mut() = true;
    }

    fn was_interrupted(&self) -> bool {
        *self.interrupted.borrow()
    }

    fn set_ui_error(&self, error: CliError) {
        *self.ui_error.borrow_mut() = Some(error);
    }

    fn take_ui_error(&self) -> Option<CliError> {
        self.ui_error.borrow_mut().take()
    }

    fn resolve_runtime_error(&self, error: recite_runtime::DialogueError) -> CliError {
        if self.was_interrupted() {
            return CliError::PlayInterrupted;
        }
        self.take_ui_error().unwrap_or_else(|| error.into())
    }
}

impl<U: PlayUiAdapter> DialogueContext for InteractiveContext<'_, U> {
    fn evaluate_condition(
        &self,
        query: ConditionQuery<'_>,
    ) -> Result<ConditionValue, ConditionEvaluationError> {
        self.ui.borrow_mut().condition(query).map_err(|error| {
            if matches!(error, CliError::PlayInterrupted) {
                self.mark_interrupted();
            }
            let message = error.to_string();
            self.set_ui_error(error);
            ConditionEvaluationError::new(message)
        })
    }
}

pub(super) trait PlayUiAdapter {
    fn message(&self, id: MsgId, args: impl IntoIterator<Item = (&'static str, String)>) -> String;
    fn start(&mut self, asset: &CompiledDialogue, block: &str) -> Result<(), CliError>;
    fn line(&mut self, line: &DialogueLine) -> Result<(), CliError>;
    fn choice(
        &mut self,
        line: Option<&DialogueLine>,
        choices: &[DialogueChoice],
    ) -> Result<ChoiceSelection, CliError>;
    fn selected_choice(&mut self, choice_id: &ChoiceId) -> Result<(), CliError>;
    fn condition(&mut self, query: ConditionQuery<'_>) -> Result<ConditionValue, CliError>;
    fn effect(&mut self, effect: &DialogueEffectRequest) -> Result<(), CliError>;
    fn acknowledge(&mut self, effect: &DialogueEffectRequest) -> Result<(), CliError>;
    fn deferred_queue(
        &mut self,
        _effects: &[DialogueEffectRequest],
        _status: DeferredQueueStatus,
    ) -> Result<(), CliError> {
        Ok(())
    }
    fn end(&mut self, deferred_effects: &[DialogueEffectRequest]) -> Result<(), CliError>;
    fn invalid_input(&mut self, message: String) -> Result<(), CliError>;
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) enum ChoiceSelection {
    Index(usize),
    Id(String),
}

impl ChoiceSelection {
    pub(super) fn parse(input: &str, messages: &Messages) -> Result<Self, CliError> {
        if input.is_empty() {
            return Err(CliError::PlayInvalidInput(
                messages.text(MsgId::PlayErrorEmptyChoice),
            ));
        }
        if let Ok(index) = input.parse::<usize>() {
            return Ok(Self::Index(index));
        }
        Ok(Self::Id(input.to_owned()))
    }
}

fn unavailable_choice_message<U: PlayUiAdapter>(
    ui: &RefCell<&mut U>,
    choice: &DialogueChoice,
) -> String {
    let ui = ui.borrow();
    match choice.unavailable_reason.as_deref() {
        Some(reason) if !reason.is_empty() => ui.message(
            MsgId::PlayErrorChoiceUnavailableReason,
            [
                ("id", choice.id.as_str().to_owned()),
                ("reason", reason.to_owned()),
            ],
        ),
        _ => ui.message(
            MsgId::PlayErrorChoiceUnavailable,
            [("id", choice.id.as_str().to_owned())],
        ),
    }
}

#[cfg(test)]
mod tests;
