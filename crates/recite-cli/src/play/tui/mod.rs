use std::{collections::BTreeMap, io};

use crossterm::event::{self, Event, KeyEventKind};
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
};
use recite_core::{ChoiceId, CompiledDialogue};
use recite_runtime::{
    ConditionExpectedType, ConditionQuery, ConditionValue, DialogueChoice, DialogueEffectMode,
    DialogueEffectRequest, DialogueLine,
};

use crate::dialogue_locale::DialogueTraversalPreview;
use crate::error::CliError;
use crate::i18n::{Messages, MsgId};
use crate::runtime_format::format_effect_arguments;
use crate::tui::{
    Keymap, PromptMode, TextBuffer, TuiIntent, TuiInteractionState, TuiSettings, enter_terminal,
    map_key, restore_terminal,
};

use super::driver::{ChoiceSelection, DeferredQueueStatus, PlayDriver, PlayUiAdapter};
use super::format::condition_query_text;
use render::render_tui;
use state::{
    TuiChoiceRow, TuiDeferredEffectRow, TuiDeferredQueueState, TuiPrompt, TuiPromptLine, TuiState,
    TuiTranscriptEntry, TuiTranscriptKind, finished_interaction, initial_choice_selection,
    initial_interaction,
};

mod interaction;
mod render;
mod state;

pub(super) fn run_tui_stdio(
    asset: &CompiledDialogue,
    block: &str,
    settings: TuiSettings,
    messages: Messages,
    dialogue_preview: Option<DialogueTraversalPreview<'_>>,
) -> Result<(), CliError> {
    let mut restore_guard = enter_terminal()?;
    let stdout = io::stdout();
    let backend = CrosstermBackend::new(stdout);
    let mut terminal = Terminal::new(backend)?;
    let mut ui = TuiPlayUi::new(&mut terminal, settings, messages);
    let driver = PlayDriver::new(asset, block);
    let result = match dialogue_preview {
        Some(preview) => driver.with_dialogue_preview(preview).run(&mut ui),
        None => driver.run(&mut ui),
    };
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
    condition_answers: BTreeMap<String, bool>,
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
            condition_answers: BTreeMap::new(),
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
}

fn cached_condition_answer(cache: &BTreeMap<String, bool>, query: &str) -> bool {
    cache.get(query).copied().unwrap_or(true)
}

fn condition_prompt(
    expected_type: ConditionExpectedType,
    query: String,
    selected_bool_answer: bool,
    keymap: Keymap,
) -> TuiPrompt {
    match expected_type {
        ConditionExpectedType::Bool => TuiPrompt::Condition {
            query,
            selected: selected_bool_answer,
            interaction: initial_interaction(keymap),
        },
        ConditionExpectedType::Enum => TuiPrompt::EnumCondition {
            query,
            interaction: initial_interaction(keymap),
            input: TextBuffer::default(),
        },
    }
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
                is_available: choice.availability.is_available,
                unavailable_reason: choice
                    .availability
                    .primary_reason
                    .as_ref()
                    .map(|reason| reason.source_text.clone()),
                is_visible: self.settings.show_unavailable_choices
                    || choice.availability.is_available,
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
            interaction: initial_interaction(self.settings.keymap),
            input: TextBuffer::default(),
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

    fn condition(&mut self, query: ConditionQuery<'_>) -> Result<ConditionValue, CliError> {
        let expected_type = query.expected_type();
        let query = condition_query_text(query);
        match expected_type {
            ConditionExpectedType::Enum => {
                self.state.prompt =
                    condition_prompt(expected_type, query.clone(), true, self.settings.keymap);
                self.state.status.clear();
                let value = self.read_enum_condition_variant()?;
                self.push(TuiTranscriptKind::Condition, Some(query), value.clone())?;
                Ok(ConditionValue::EnumVariant(value))
            }
            ConditionExpectedType::Bool => {
                let selected = cached_condition_answer(&self.condition_answers, &query);
                self.state.prompt =
                    condition_prompt(expected_type, query.clone(), selected, self.settings.keymap);
                self.state.status.clear();
                let answer = self.read_condition_selection()?;
                self.condition_answers.insert(query.clone(), answer);
                self.push(
                    TuiTranscriptKind::Condition,
                    Some(query),
                    answer.to_string(),
                )?;
                Ok(ConditionValue::Bool(answer))
            }
        }
    }

    fn effect(&mut self, effect: &DialogueEffectRequest) -> Result<(), CliError> {
        let args = format_effect_arguments(&effect.args);
        self.state.prompt = TuiPrompt::Effect {
            mode: effect.mode.to_string(),
            id: effect.id.as_str().to_owned(),
            function: effect.function.clone(),
            args: args.clone(),
            interaction: TuiInteractionState::new(PromptMode::Insert),
            input: TextBuffer::default(),
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
            DeferredQueueStatus::Ready => Some(TuiDeferredQueueState::Ready),
        };
        self.render()
    }

    fn end(&mut self, deferred_effects: &[DialogueEffectRequest]) -> Result<(), CliError> {
        self.state.prompt = TuiPrompt::Finished {
            interaction: finished_interaction(),
        };
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
mod tests;
