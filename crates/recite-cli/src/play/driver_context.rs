use std::cell::RefCell;

use recite_core::ChoiceId;
use recite_runtime::{
    ConditionEvaluationError, ConditionQuery, ConditionValue, DialogueChoice, DialogueContext,
    DialogueEffectRequest, DialogueLine,
};
use recite_ui::UiArg;
use recite_ui::UiArgs;

use crate::error::CliError;
use crate::i18n::MsgId;

use super::driver::{DeferredQueueStatus, PlayUiAdapter};
use super::driver_api::ChoiceSelection;

pub(super) struct InteractiveContext<'a, U> {
    ui: RefCell<&'a mut U>,
    interrupted: RefCell<bool>,
    ui_error: RefCell<Option<CliError>>,
}

impl<'a, U> InteractiveContext<'a, U> {
    pub(super) fn new(ui: &'a mut U) -> Self {
        Self {
            ui: RefCell::new(ui),
            interrupted: RefCell::new(false),
            ui_error: RefCell::new(None),
        }
    }
}

impl<U: PlayUiAdapter> InteractiveContext<'_, U> {
    pub(super) fn line(&self, line: &DialogueLine) -> Result<(), CliError> {
        self.ui.borrow_mut().line(line)
    }
    pub(super) fn choice(
        &self,
        line: Option<&DialogueLine>,
        choices: &[DialogueChoice],
    ) -> Result<ChoiceId, CliError> {
        loop {
            let selection = match self.ui.borrow_mut().choice(line, choices) {
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
                        if choice.availability.is_available {
                            return Ok(choice.id.clone());
                        }
                        self.ui
                            .borrow_mut()
                            .invalid_input(unavailable_choice_message(&self.ui, choice))?;
                    } else if index == 0 || index > choices.len() {
                        let message = self.ui.borrow().message_typed(
                            MsgId::PlayErrorChoiceIndexOutOfRange,
                            UiArgs::from([
                                ("index".to_owned(), UiArg::from(index)),
                                ("count".to_owned(), UiArg::from(choices.len())),
                            ]),
                        );
                        self.ui.borrow_mut().invalid_input(message)?;
                    } else {
                        let choice = &choices[index - 1];
                        if choice.availability.is_available {
                            return Ok(choice.id.clone());
                        }
                        self.ui
                            .borrow_mut()
                            .invalid_input(unavailable_choice_message(&self.ui, choice))?;
                    }
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
                    let Some(choice) = choices.iter().find(|choice| choice.id == choice_id) else {
                        let message = self
                            .ui
                            .borrow()
                            .message(MsgId::PlayErrorChoiceIdUnavailable, [("id", id)]);
                        self.ui.borrow_mut().invalid_input(message)?;
                        continue;
                    };
                    if choice.availability.is_available {
                        return Ok(choice_id);
                    }
                    self.ui
                        .borrow_mut()
                        .invalid_input(unavailable_choice_message(&self.ui, choice))?;
                }
            }
        }
    }
    pub(super) fn selected_choice(&self, id: &ChoiceId) -> Result<(), CliError> {
        self.ui.borrow_mut().selected_choice(id)
    }
    pub(super) fn effect(&self, effect: &DialogueEffectRequest) -> Result<(), CliError> {
        self.ui.borrow_mut().effect(effect)
    }
    pub(super) fn acknowledge(&self, effect: &DialogueEffectRequest) -> Result<(), CliError> {
        self.ui.borrow_mut().acknowledge(effect)
    }
    pub(super) fn deferred_queue(
        &self,
        effects: &[DialogueEffectRequest],
        status: DeferredQueueStatus,
    ) -> Result<(), CliError> {
        self.ui.borrow_mut().deferred_queue(effects, status)
    }
    pub(super) fn end(&self, effects: &[DialogueEffectRequest]) -> Result<(), CliError> {
        self.ui.borrow_mut().end(effects)
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
    pub(super) fn resolve_runtime_error(&self, error: recite_runtime::DialogueError) -> CliError {
        if self.was_interrupted() {
            return CliError::PlayInterrupted;
        }
        self.take_ui_error().unwrap_or(CliError::Runtime(error))
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

fn unavailable_choice_message<U: PlayUiAdapter>(
    ui: &RefCell<&mut U>,
    choice: &DialogueChoice,
) -> String {
    match choice
        .availability
        .primary_reason
        .as_ref()
        .map(|reason| reason.source_text.as_str())
    {
        Some(reason) if !reason.is_empty() => ui.borrow().message(
            MsgId::PlayErrorChoiceUnavailableReason,
            [
                ("id", choice.id.as_str().to_owned()),
                ("reason", reason.to_owned()),
            ],
        ),
        _ => ui.borrow().message(
            MsgId::PlayErrorChoiceUnavailable,
            [("id", choice.id.as_str().to_owned())],
        ),
    }
}
