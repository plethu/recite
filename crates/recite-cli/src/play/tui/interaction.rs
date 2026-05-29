use ratatui::backend::Backend;

use crate::error::CliError;
use crate::i18n::MsgId;
use crate::tui::{GlobalAction, Keymap, PromptMode, TuiIntent, command_quits};

use super::super::driver::ChoiceSelection;
use super::TuiPlayUi;
use super::state::{
    close_help, condition_selection, move_choice_selection, move_condition_selection,
    mutate_prompt_command, mutate_prompt_input, prompt_command, prompt_input, prompt_label,
    prompt_mode, selected_choice_id, set_condition_selection, set_prompt_mode,
    start_prompt_command, toggle_deferred_queue, toggle_help,
};

impl<B: Backend> TuiPlayUi<'_, B> {
    pub(super) fn wait_for_exit(&mut self) -> Result<(), CliError> {
        loop {
            let mode = prompt_mode(&self.state.prompt);
            let intent = self.read_intent(mode)?;
            match self.handle_global_prompt_intent(mode, intent)? {
                PromptIntentStatus::Quit => return Ok(()),
                PromptIntentStatus::Consumed => continue,
                PromptIntentStatus::Continue => {}
            }
            if intent == TuiIntent::Submit {
                return Ok(());
            }
        }
    }

    fn read_command(&mut self) -> Result<bool, CliError> {
        let previous_prompt = self.state.prompt.clone();
        let previous_status = self.state.status.clone();
        start_prompt_command(&mut self.state.prompt);
        self.state.status = self.messages.text(MsgId::TuiCommand);
        loop {
            match self.read_intent(PromptMode::Command)? {
                TuiIntent::Submit => {
                    let command = prompt_command(&self.state.prompt).to_owned();
                    self.state.prompt = previous_prompt;
                    if command_quits(&command) {
                        return Ok(true);
                    }
                    self.state.status = self
                        .messages
                        .format(MsgId::TuiUnknownCommand, [("command", command)]);
                    return Ok(false);
                }
                TuiIntent::Quit => return Err(CliError::PlayInterrupted),
                TuiIntent::Cancel => {
                    self.state.prompt = previous_prompt;
                    self.state.status = previous_status;
                    return Ok(false);
                }
                TuiIntent::ToggleAuxiliaryPanel => {
                    toggle_deferred_queue(&mut self.state);
                }
                intent => {
                    mutate_prompt_command(&mut self.state.prompt, intent);
                    self.state.status = self.messages.format(
                        MsgId::TuiCommandWithValue,
                        [("command", prompt_command(&self.state.prompt).to_owned())],
                    );
                }
            }
        }
    }

    fn handle_global_prompt_intent(
        &mut self,
        mode: PromptMode,
        intent: TuiIntent,
    ) -> Result<PromptIntentStatus, CliError> {
        match crate::tui::global_action(mode, intent) {
            Some(GlobalAction::Quit) => Ok(PromptIntentStatus::Quit),
            Some(GlobalAction::OpenCommand) => {
                if self.read_command()? {
                    Ok(PromptIntentStatus::Quit)
                } else {
                    Ok(PromptIntentStatus::Consumed)
                }
            }
            Some(GlobalAction::ToggleHelp) => {
                toggle_help(&mut self.state.prompt);
                Ok(PromptIntentStatus::Consumed)
            }
            Some(GlobalAction::ToggleAuxiliaryPanel) => {
                toggle_deferred_queue(&mut self.state);
                Ok(PromptIntentStatus::Consumed)
            }
            Some(GlobalAction::CloseHelp) => {
                close_help(&mut self.state.prompt);
                Ok(PromptIntentStatus::Consumed)
            }
            None => Ok(PromptIntentStatus::Continue),
        }
    }

    pub(super) fn read_choice_selection(&mut self) -> Result<ChoiceSelection, CliError> {
        self.read_prompt_response(|ui, mode, intent| {
            match intent {
                TuiIntent::Submit => {
                    let input = prompt_input(&ui.state.prompt).trim().to_owned();
                    if !input.is_empty() {
                        return ChoiceSelection::parse(&input, &ui.messages)
                            .map(PromptAction::Return);
                    }
                    if let Some(id) = selected_choice_id(&ui.state.prompt) {
                        return Ok(PromptAction::Return(ChoiceSelection::Id(id.to_owned())));
                    }
                    return Err(CliError::PlayInvalidInput(
                        ui.messages.text(MsgId::PlayErrorEmptyChoice),
                    ));
                }
                TuiIntent::MoveNext => {
                    move_choice_selection(&mut ui.state.prompt, 1);
                    ui.state.status.clear();
                }
                TuiIntent::MovePrevious => {
                    move_choice_selection(&mut ui.state.prompt, -1);
                    ui.state.status.clear();
                }
                TuiIntent::StartInsert => {
                    set_prompt_mode(&mut ui.state.prompt, PromptMode::Insert);
                    ui.state.status = prompt_label(ui.messages.text(MsgId::TuiChoiceInputPrefix));
                }
                TuiIntent::Cancel => {
                    if ui.settings.keymap == Keymap::Vim && mode == PromptMode::Insert {
                        set_prompt_mode(&mut ui.state.prompt, PromptMode::Normal);
                        ui.state.status.clear();
                    }
                }
                intent => {
                    mutate_prompt_input(&mut ui.state.prompt, intent);
                    let input = prompt_input(&ui.state.prompt);
                    if input.is_empty() {
                        ui.state.status.clear();
                    } else {
                        ui.state.status = ui
                            .messages
                            .format(MsgId::TuiChoiceInput, [("input", input.to_owned())]);
                    }
                }
            }
            Ok(PromptAction::Continue)
        })
    }

    pub(super) fn read_condition_selection(&mut self) -> Result<bool, CliError> {
        self.read_prompt_response(|ui, mode, intent| {
            match intent {
                TuiIntent::Submit => {
                    return condition_selection(&ui.state.prompt)
                        .map(PromptAction::Return)
                        .ok_or_else(|| {
                            CliError::PlayInvalidInput(ui.messages.text(MsgId::PlayErrorEnterYOrN))
                        });
                }
                TuiIntent::MoveNext | TuiIntent::MovePrevious => {
                    move_condition_selection(&mut ui.state.prompt);
                    ui.state.status.clear();
                }
                TuiIntent::Text(ch)
                    if ui.settings.keymap == Keymap::Standard
                        && matches!(ch, 'y' | 'Y' | 'n' | 'N') =>
                {
                    let value = matches!(ch, 'y' | 'Y');
                    set_condition_selection(&mut ui.state.prompt, value);
                    return Ok(PromptAction::Return(value));
                }
                TuiIntent::Cancel
                    if ui.settings.keymap == Keymap::Vim && mode == PromptMode::Insert =>
                {
                    set_prompt_mode(&mut ui.state.prompt, PromptMode::Normal);
                    ui.state.status.clear();
                }
                TuiIntent::StartInsert => {
                    set_prompt_mode(&mut ui.state.prompt, PromptMode::Insert);
                }
                _ => {}
            }
            Ok(PromptAction::Continue)
        })
    }

    pub(super) fn read_enum_condition_variant(&mut self) -> Result<String, CliError> {
        self.read_prompt_response(|ui, mode, intent| {
            match intent {
                TuiIntent::Submit => {
                    let input = prompt_input(&ui.state.prompt);
                    match enum_condition_variant(input, &ui.messages) {
                        Ok(value) => return Ok(PromptAction::Return(value)),
                        Err(CliError::PlayInvalidInput(message)) => {
                            ui.state.status = ui
                                .messages
                                .format(MsgId::PlayInvalidInput, [("message", message)]);
                        }
                        Err(error) => return Err(error),
                    }
                }
                TuiIntent::Cancel
                    if ui.settings.keymap == Keymap::Vim && mode == PromptMode::Insert =>
                {
                    set_prompt_mode(&mut ui.state.prompt, PromptMode::Normal);
                    ui.state.status.clear();
                }
                TuiIntent::StartInsert => {
                    set_prompt_mode(&mut ui.state.prompt, PromptMode::Insert);
                    ui.state.status = prompt_label(ui.messages.text(MsgId::TuiInputEnumVariant));
                }
                intent => {
                    mutate_prompt_input(&mut ui.state.prompt, intent);
                    let input = prompt_input(&ui.state.prompt);
                    if input.is_empty() {
                        ui.state.status.clear();
                    } else {
                        ui.state.status = ui
                            .messages
                            .format(MsgId::TuiEnumVariantInput, [("input", input.to_owned())]);
                    }
                }
            }
            Ok(PromptAction::Continue)
        })
    }

    pub(super) fn read_effect_acknowledgement(&mut self) -> Result<(), CliError> {
        self.read_prompt_response(|ui, mode, intent| {
            match intent {
                TuiIntent::Submit => return Ok(PromptAction::Return(())),
                TuiIntent::Cancel
                    if ui.settings.keymap == Keymap::Vim && mode == PromptMode::Insert =>
                {
                    set_prompt_mode(&mut ui.state.prompt, PromptMode::Normal);
                    ui.state.status.clear();
                }
                TuiIntent::StartInsert => {
                    set_prompt_mode(&mut ui.state.prompt, PromptMode::Insert);
                }
                _ => {}
            }
            Ok(PromptAction::Continue)
        })
    }

    fn read_prompt_response<T>(
        &mut self,
        mut handle: impl FnMut(&mut Self, PromptMode, TuiIntent) -> Result<PromptAction<T>, CliError>,
    ) -> Result<T, CliError> {
        loop {
            let mode = prompt_mode(&self.state.prompt);
            let intent = self.read_intent(mode)?;
            match self.handle_global_prompt_intent(mode, intent)? {
                PromptIntentStatus::Quit => return Err(CliError::PlayInterrupted),
                PromptIntentStatus::Consumed => continue,
                PromptIntentStatus::Continue => {}
            }
            if mode == PromptMode::Help && intent == TuiIntent::Submit {
                continue;
            }
            match handle(self, mode, intent)? {
                PromptAction::Continue => {}
                PromptAction::Return(value) => return Ok(value),
            }
        }
    }
}

pub(super) fn enum_condition_variant(
    input: &str,
    messages: &crate::i18n::Messages,
) -> Result<String, CliError> {
    let value = input.trim().to_owned();
    if value.is_empty() {
        return Err(CliError::PlayInvalidInput(
            messages.text(MsgId::PlayErrorEnterEnumVariant),
        ));
    }
    Ok(value)
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PromptIntentStatus {
    Continue,
    Consumed,
    Quit,
}

enum PromptAction<T> {
    Continue,
    Return(T),
}
