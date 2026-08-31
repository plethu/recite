use std::{collections::BTreeMap, io};

use crossterm::event::{self, Event, KeyEventKind};
use ratatui::{
    Terminal,
    backend::{Backend, CrosstermBackend},
};
use recite_core::CompiledDialogue;

use crate::dialogue_locale::DialogueTraversalPreview;
use crate::error::CliError;
use crate::i18n::Messages;
use crate::tui::{PromptMode, TuiIntent, TuiSettings, enter_terminal, map_key, restore_terminal};

use super::preview::run_preview;
use render::render_tui;
use state::{TuiState, TuiTranscriptEntry, TuiTranscriptKind};

mod choice;
mod interaction;
mod preview;
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
    let result = run_preview(asset, block, dialogue_preview, &mut ui);
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
    /// Remembers the last displayed boolean answer for prompt ergonomics only.
    /// PreviewSession remains the sole condition/traversal authority.
    condition_answers: BTreeMap<String, bool>,
}

impl<'a, B: Backend> TuiPlayUi<'a, B> {
    fn new(terminal: &'a mut Terminal<B>, settings: TuiSettings, messages: Messages) -> Self {
        let state = TuiState {
            key_hints: settings.key_hints,
            keymap: settings.keymap,
            palette: crate::tui::TuiPalette::from_settings(&settings),
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

#[cfg(test)]
mod tests;
