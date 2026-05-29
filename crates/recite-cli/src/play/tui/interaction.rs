use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::backend::Backend;

use crate::error::CliError;
use crate::i18n::MsgId;
use crate::tui::{Keymap, PromptMode, TextBuffer, TuiIntent, command_quits};

use super::super::driver::ChoiceSelection;
use super::TuiPlayUi;
use super::state::{
    close_help, condition_selection, move_choice_selection, move_condition_selection,
    mutate_prompt_command, mutate_prompt_input, prompt_command, prompt_input, prompt_label,
    prompt_mode, selected_choice_id, set_command, set_condition_selection, set_prompt_mode,
    toggle_deferred_queue, toggle_help,
};

impl<B: Backend> TuiPlayUi<'_, B> {
    pub(super) fn wait_for_exit(&mut self) -> Result<(), CliError> {
        let mut command = TextBuffer::default();
        let mut command_mode = false;
        loop {
            self.render()?;
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && matches!(key.code, KeyCode::Char('c') | KeyCode::Char('C'))
                {
                    return Ok(());
                }
                if key.modifiers.contains(KeyModifiers::CONTROL)
                    && matches!(key.code, KeyCode::Char('d') | KeyCode::Char('D'))
                {
                    toggle_deferred_queue(&mut self.state);
                    continue;
                }
                if command_mode {
                    match key.code {
                        KeyCode::Enter if command_quits(command.as_str()) => return Ok(()),
                        KeyCode::Esc => {
                            command_mode = false;
                            command.clear();
                            self.state.status = self.messages.text(MsgId::TuiFinished);
                        }
                        KeyCode::Char(ch) => {
                            command.insert(ch);
                            self.state.status = self.messages.format(
                                MsgId::TuiCommandWithValue,
                                [("command", command.as_str().to_owned())],
                            );
                        }
                        KeyCode::Backspace => {
                            command.backspace();
                            self.state.status = self.messages.format(
                                MsgId::TuiCommandWithValue,
                                [("command", command.as_str().to_owned())],
                            );
                        }
                        _ => {}
                    }
                    continue;
                }
                if prompt_mode(&self.state.prompt) == PromptMode::Help {
                    match key.code {
                        KeyCode::Esc | KeyCode::Char('?') => {
                            close_help(&mut self.state.prompt);
                            continue;
                        }
                        KeyCode::Char('q') => return Ok(()),
                        _ => {}
                    }
                }
                match key.code {
                    KeyCode::Enter | KeyCode::Esc | KeyCode::Char('q') => return Ok(()),
                    KeyCode::Char('?') => {
                        toggle_help(&mut self.state.prompt);
                    }
                    KeyCode::Char(':') if self.settings.keymap == Keymap::Vim => {
                        command_mode = true;
                        command.clear();
                        self.state.status = self.messages.text(MsgId::TuiCommand);
                    }
                    _ => {}
                }
            }
        }
    }

    fn read_command(&mut self) -> Result<bool, CliError> {
        let previous_prompt = self.state.prompt.clone();
        set_prompt_mode(&mut self.state.prompt, PromptMode::Command);
        set_command(&mut self.state.prompt, TextBuffer::default());
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
                    return Ok(false);
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
        match intent {
            TuiIntent::Quit => Err(CliError::PlayInterrupted),
            TuiIntent::OpenCommand => {
                if self.read_command()? {
                    Ok(PromptIntentStatus::Quit)
                } else {
                    Ok(PromptIntentStatus::Consumed)
                }
            }
            TuiIntent::ToggleHelp => {
                toggle_help(&mut self.state.prompt);
                Ok(PromptIntentStatus::Consumed)
            }
            TuiIntent::ToggleDeferredQueue => {
                toggle_deferred_queue(&mut self.state);
                Ok(PromptIntentStatus::Consumed)
            }
            TuiIntent::Cancel if mode == PromptMode::Help => {
                close_help(&mut self.state.prompt);
                Ok(PromptIntentStatus::Consumed)
            }
            _ => Ok(PromptIntentStatus::Continue),
        }
    }

    pub(super) fn read_choice_selection(&mut self) -> Result<ChoiceSelection, CliError> {
        loop {
            let mode = prompt_mode(&self.state.prompt);
            let intent = self.read_intent(mode)?;
            match self.handle_global_prompt_intent(mode, intent)? {
                PromptIntentStatus::Quit => return Err(CliError::PlayInterrupted),
                PromptIntentStatus::Consumed => continue,
                PromptIntentStatus::Continue => {}
            }
            match intent {
                TuiIntent::Submit => {
                    let input = prompt_input(&self.state.prompt).trim().to_owned();
                    if !input.is_empty() {
                        return ChoiceSelection::parse(&input, &self.messages);
                    }
                    if let Some(id) = selected_choice_id(&self.state.prompt) {
                        return Ok(ChoiceSelection::Id(id.to_owned()));
                    }
                    return Err(CliError::PlayInvalidInput(
                        self.messages.text(MsgId::PlayErrorEmptyChoice),
                    ));
                }
                TuiIntent::MoveNext => {
                    move_choice_selection(&mut self.state.prompt, 1);
                    self.state.status.clear();
                }
                TuiIntent::MovePrevious => {
                    move_choice_selection(&mut self.state.prompt, -1);
                    self.state.status.clear();
                }
                TuiIntent::StartInsert => {
                    set_prompt_mode(&mut self.state.prompt, PromptMode::Insert);
                    self.state.status =
                        prompt_label(self.messages.text(MsgId::TuiChoiceInputPrefix));
                }
                TuiIntent::Cancel => {
                    if self.settings.keymap == Keymap::Vim && mode == PromptMode::Insert {
                        set_prompt_mode(&mut self.state.prompt, PromptMode::Normal);
                        self.state.status.clear();
                    }
                }
                intent => {
                    mutate_prompt_input(&mut self.state.prompt, intent);
                    let input = prompt_input(&self.state.prompt);
                    if input.is_empty() {
                        self.state.status.clear();
                    } else {
                        self.state.status = self
                            .messages
                            .format(MsgId::TuiChoiceInput, [("input", input.to_owned())]);
                    }
                }
            }
        }
    }

    pub(super) fn read_condition_selection(&mut self) -> Result<bool, CliError> {
        loop {
            let mode = prompt_mode(&self.state.prompt);
            let intent = self.read_intent(mode)?;
            match self.handle_global_prompt_intent(mode, intent)? {
                PromptIntentStatus::Quit => return Err(CliError::PlayInterrupted),
                PromptIntentStatus::Consumed => continue,
                PromptIntentStatus::Continue => {}
            }
            match intent {
                TuiIntent::Submit => {
                    return condition_selection(&self.state.prompt).ok_or_else(|| {
                        CliError::PlayInvalidInput(self.messages.text(MsgId::PlayErrorEnterYOrN))
                    });
                }
                TuiIntent::MoveNext | TuiIntent::MovePrevious => {
                    move_condition_selection(&mut self.state.prompt);
                    self.state.status.clear();
                }
                TuiIntent::Text(ch)
                    if self.settings.keymap == Keymap::Standard
                        && matches!(ch, 'y' | 'Y' | 'n' | 'N') =>
                {
                    let value = matches!(ch, 'y' | 'Y');
                    set_condition_selection(&mut self.state.prompt, value);
                    return Ok(value);
                }
                TuiIntent::Cancel
                    if self.settings.keymap == Keymap::Vim && mode == PromptMode::Insert =>
                {
                    set_prompt_mode(&mut self.state.prompt, PromptMode::Normal);
                    self.state.status.clear();
                }
                TuiIntent::StartInsert => {
                    set_prompt_mode(&mut self.state.prompt, PromptMode::Insert);
                }
                _ => {}
            }
        }
    }

    pub(super) fn read_enum_condition_variant(&mut self) -> Result<String, CliError> {
        loop {
            let mode = prompt_mode(&self.state.prompt);
            let intent = self.read_intent(mode)?;
            match self.handle_global_prompt_intent(mode, intent)? {
                PromptIntentStatus::Quit => return Err(CliError::PlayInterrupted),
                PromptIntentStatus::Consumed => continue,
                PromptIntentStatus::Continue => {}
            }
            match intent {
                TuiIntent::Submit => {
                    let input = prompt_input(&self.state.prompt);
                    match enum_condition_variant(input, &self.messages) {
                        Ok(value) => return Ok(value),
                        Err(CliError::PlayInvalidInput(message)) => {
                            self.state.status = self
                                .messages
                                .format(MsgId::PlayInvalidInput, [("message", message)]);
                            continue;
                        }
                        Err(error) => return Err(error),
                    }
                }
                TuiIntent::Cancel
                    if self.settings.keymap == Keymap::Vim && mode == PromptMode::Insert =>
                {
                    set_prompt_mode(&mut self.state.prompt, PromptMode::Normal);
                    self.state.status.clear();
                }
                TuiIntent::StartInsert => {
                    set_prompt_mode(&mut self.state.prompt, PromptMode::Insert);
                    self.state.status =
                        prompt_label(self.messages.text(MsgId::TuiInputEnumVariant));
                }
                intent => {
                    mutate_prompt_input(&mut self.state.prompt, intent);
                    let input = prompt_input(&self.state.prompt);
                    if input.is_empty() {
                        self.state.status.clear();
                    } else {
                        self.state.status = self
                            .messages
                            .format(MsgId::TuiEnumVariantInput, [("input", input.to_owned())]);
                    }
                }
            }
        }
    }

    pub(super) fn read_effect_acknowledgement(&mut self) -> Result<(), CliError> {
        loop {
            let mode = prompt_mode(&self.state.prompt);
            let intent = self.read_intent(mode)?;
            match self.handle_global_prompt_intent(mode, intent)? {
                PromptIntentStatus::Quit => return Err(CliError::PlayInterrupted),
                PromptIntentStatus::Consumed => continue,
                PromptIntentStatus::Continue => {}
            }
            match intent {
                TuiIntent::Submit => return Ok(()),
                TuiIntent::Cancel
                    if self.settings.keymap == Keymap::Vim && mode == PromptMode::Insert =>
                {
                    set_prompt_mode(&mut self.state.prompt, PromptMode::Normal);
                    self.state.status.clear();
                }
                TuiIntent::StartInsert => {
                    set_prompt_mode(&mut self.state.prompt, PromptMode::Insert);
                }
                _ => {}
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
