use std::{collections::HashMap, io};

use crossterm::event::{self, Event, KeyCode, KeyEventKind, KeyModifiers};
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
};
use recite_core::{ChoiceId, CompiledDialogue};
use recite_runtime::{
    ConditionQuery, DialogueChoice, DialogueEffectMode, DialogueEffectRequest, DialogueLine,
};

use crate::error::CliError;
use crate::i18n::{Messages, MsgId};
use crate::runtime_format::format_effect_arguments;
use crate::tui::{
    Keymap, PromptMode, TextBuffer, TuiIntent, TuiSettings, command_quits, enter_terminal, map_key,
    restore_terminal,
};

use super::driver::{ChoiceSelection, DeferredQueueStatus, PlayDriver, PlayUiAdapter};
use super::format::condition_query_text;
use render::render_tui;
use state::{
    TuiChoiceRow, TuiDeferredEffectRow, TuiDeferredQueueState, TuiPrompt, TuiPromptLine, TuiState,
    TuiTranscriptEntry, TuiTranscriptKind, close_help, condition_selection,
    initial_choice_selection, initial_prompt_mode, move_choice_selection, move_condition_selection,
    mutate_prompt_command, mutate_prompt_input, prompt_command, prompt_input, prompt_label,
    prompt_mode, selected_choice_id, set_command, set_condition_selection, set_prompt_mode,
    toggle_deferred_queue, toggle_help,
};

mod render;
mod state;

pub(super) fn run_tui_stdio(
    asset: &CompiledDialogue,
    block: &str,
    settings: TuiSettings,
    messages: Messages,
) -> Result<(), CliError> {
    let mut restore_guard = enter_terminal()?;
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut ui = TuiPlayUi::new(&mut terminal, settings, messages);
    let result = PlayDriver::new(asset, block).run(&mut ui);
    let restore_result = restore_terminal(&mut terminal);
    if restore_result.is_ok() {
        restore_guard.disarm();
    }
    match (result, restore_result) {
        (Err(CliError::PlayInterrupted), Ok(())) => Ok(()),
        (result, Ok(())) => result,
        (_, Err(error)) => Err(error),
    }
}

struct TuiPlayUi<'a, B: Backend> {
    terminal: &'a mut Terminal<B>,
    state: TuiState,
    settings: TuiSettings,
    messages: Messages,
    condition_answers: HashMap<String, bool>,
}

impl<'a, B: Backend> TuiPlayUi<'a, B> {
    fn new(terminal: &'a mut Terminal<B>, settings: TuiSettings, messages: Messages) -> Self {
        let state = TuiState {
            key_hints: settings.key_hints,
            keymap: settings.keymap,
            ..TuiState::default()
        };
        Self {
            terminal,
            state,
            settings,
            messages,
            condition_answers: HashMap::new(),
        }
    }

    fn push(
        &mut self,
        kind: TuiTranscriptKind,
        id: Option<String>,
        text: impl Into<String>,
    ) -> Result<(), CliError> {
        self.state.transcript.push(TuiTranscriptEntry {
            kind,
            id,
            text: text.into(),
        });
        self.render()
    }

    fn render(&mut self) -> Result<(), CliError> {
        let state = &self.state;
        let messages = &self.messages;
        self.terminal
            .draw(|frame| render_tui(frame, state, messages))?;
        Ok(())
    }

    fn read_intent(&mut self, mode: PromptMode) -> Result<TuiIntent, CliError> {
        loop {
            self.render()?;
            if let Event::Key(key) = event::read()? {
                if key.kind != KeyEventKind::Press {
                    continue;
                }
                return Ok(map_key(self.settings.keymap, mode, key));
            }
        }
    }

    fn wait_for_exit(&mut self) -> Result<(), CliError> {
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

    fn read_choice_selection(&mut self) -> Result<ChoiceSelection, CliError> {
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

    fn read_condition_selection(&mut self) -> Result<bool, CliError> {
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

    fn read_effect_acknowledgement(&mut self) -> Result<(), CliError> {
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum PromptIntentStatus {
    Continue,
    Consumed,
    Quit,
}

fn cached_condition_answer(cache: &HashMap<String, bool>, query: &str) -> bool {
    cache.get(query).copied().unwrap_or(true)
}

impl<B: Backend> PlayUiAdapter for TuiPlayUi<'_, B> {
    fn message(&self, id: MsgId, args: impl IntoIterator<Item = (&'static str, String)>) -> String {
        self.messages.format(id, args)
    }

    fn start(&mut self, asset: &CompiledDialogue, block: &str) -> Result<(), CliError> {
        self.state.asset = asset.header.asset_id.as_str().to_owned();
        self.state.block = block.to_owned();
        self.state.status = self.messages.text(MsgId::TuiReady);
        self.render()
    }

    fn line(&mut self, line: &DialogueLine) -> Result<(), CliError> {
        self.state.prompt = TuiPrompt::None;
        self.push(
            TuiTranscriptKind::Line,
            Some(line.id.as_str().to_owned()),
            line.text.clone(),
        )
    }

    fn choice(
        &mut self,
        line: Option<&DialogueLine>,
        choices: &[DialogueChoice],
    ) -> Result<ChoiceSelection, CliError> {
        let rows = choices
            .iter()
            .enumerate()
            .map(|(index, choice)| TuiChoiceRow {
                index: index + 1,
                id: choice.id.as_str().to_owned(),
                text: choice.text.clone(),
                is_available: choice.is_available,
                unavailable_reason: choice.unavailable_reason.clone(),
                is_visible: self.settings.show_unavailable_choices || choice.is_available,
            })
            .collect::<Vec<_>>();
        let selected = initial_choice_selection(&rows);
        self.state.prompt = TuiPrompt::Choice {
            line: line.map(|line| TuiPromptLine {
                id: line.id.as_str().to_owned(),
                text: line.text.clone(),
            }),
            choices: rows,
            selected,
            mode: initial_prompt_mode(self.settings.keymap),
            input: TextBuffer::default(),
            command: TextBuffer::default(),
            show_help: false,
        };
        self.state.status.clear();
        self.read_choice_selection()
    }

    fn selected_choice(&mut self, choice_id: &ChoiceId) -> Result<(), CliError> {
        if let TuiPrompt::Choice {
            line: Some(line), ..
        } = &self.state.prompt
        {
            self.state.transcript.push(TuiTranscriptEntry {
                kind: TuiTranscriptKind::Prompt,
                id: Some(line.id.clone()),
                text: line.text.clone(),
            });
        }
        self.push(
            TuiTranscriptKind::Choice,
            Some(choice_id.as_str().to_owned()),
            String::new(),
        )
    }

    fn condition(&mut self, query: ConditionQuery<'_>) -> Result<bool, CliError> {
        let query = condition_query_text(query);
        let selected = cached_condition_answer(&self.condition_answers, &query);
        self.state.prompt = TuiPrompt::Condition {
            query: query.clone(),
            selected,
            mode: initial_prompt_mode(self.settings.keymap),
            command: TextBuffer::default(),
            show_help: false,
        };
        self.state.status.clear();
        let answer = self.read_condition_selection()?;
        self.condition_answers.insert(query.clone(), answer);
        self.push(
            TuiTranscriptKind::Condition,
            Some(query),
            answer.to_string(),
        )?;
        Ok(answer)
    }

    fn effect(&mut self, effect: &DialogueEffectRequest) -> Result<(), CliError> {
        let args = format_effect_arguments(&effect.args);
        self.state.prompt = TuiPrompt::Effect {
            mode: effect.mode.to_string(),
            id: effect.id.as_str().to_owned(),
            function: effect.function.clone(),
            args: args.clone(),
            input_mode: PromptMode::Insert,
            input: TextBuffer::default(),
            command: TextBuffer::default(),
            show_help: false,
        };
        self.push(
            TuiTranscriptKind::Effect,
            Some(effect.id.as_str().to_owned()),
            format!("{} {} {}", effect.mode, effect.function, args),
        )
    }

    fn acknowledge(&mut self, effect: &DialogueEffectRequest) -> Result<(), CliError> {
        self.state.status.clear();
        self.read_effect_acknowledgement()?;
        self.push(
            TuiTranscriptKind::Ack,
            Some(effect.id.as_str().to_owned()),
            self.messages.text(MsgId::TuiTranscriptCompleted),
        )?;
        Ok(())
    }

    fn deferred_queue(
        &mut self,
        effects: &[DialogueEffectRequest],
        status: DeferredQueueStatus,
    ) -> Result<(), CliError> {
        self.state.deferred_queue = effects
            .iter()
            .map(|effect| TuiDeferredEffectRow {
                id: effect.id.as_str().to_owned(),
                function: effect.function.clone(),
                args: format_effect_arguments(&effect.args),
            })
            .collect();
        self.state.deferred_queue_state = match status {
            DeferredQueueStatus::Scheduled => Some(TuiDeferredQueueState::Scheduled),
            DeferredQueueStatus::Dispatched => Some(TuiDeferredQueueState::Dispatched),
        };
        self.render()
    }

    fn end(&mut self, deferred_effects: &[DialogueEffectRequest]) -> Result<(), CliError> {
        self.state.prompt = TuiPrompt::Finished { show_help: false };
        self.push(TuiTranscriptKind::End, None, String::new())?;
        if !deferred_effects.is_empty() {
            self.state.transcript.push(TuiTranscriptEntry {
                kind: TuiTranscriptKind::Deferred,
                id: None,
                text: self.messages.text(MsgId::TuiTranscriptDeferredEffects),
            });
            for effect in deferred_effects {
                self.state.transcript.push(TuiTranscriptEntry {
                    kind: TuiTranscriptKind::Deferred,
                    id: Some(effect.id.as_str().to_owned()),
                    text: format!(
                        "{} {} {}",
                        DialogueEffectMode::Deferred,
                        effect.function,
                        format_effect_arguments(&effect.args)
                    ),
                });
            }
            self.render()?;
        }
        self.state.status = self.messages.text(MsgId::TuiFinished);
        self.wait_for_exit()
    }

    fn invalid_input(&mut self, message: String) -> Result<(), CliError> {
        self.state.status = self
            .messages
            .format(MsgId::PlayInvalidInput, [("message", message)]);
        self.render()
    }
}

#[cfg(test)]
#[path = "mod_tests.rs"]
mod tests;
