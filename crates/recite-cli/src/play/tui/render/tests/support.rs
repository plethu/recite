use ratatui::Terminal;

use crate::i18n::Messages;
use crate::tui::{KeyHints, Keymap, PromptMode, TextBuffer};

use super::super::super::state::{TuiChoiceRow, TuiPrompt, TuiState};
use super::super::{controls, render_tui};

pub(super) fn choice_help_state(keymap: Keymap) -> TuiState {
    TuiState {
        asset: "asset".to_owned(),
        block: "start".to_owned(),
        transcript: Vec::new(),
        prompt: choice_prompt(true),
        status: "choice".to_owned(),
        key_hints: KeyHints::Contextual,
        keymap,
        ..TuiState::default()
    }
}

pub(super) fn choice_prompt(show_help: bool) -> TuiPrompt {
    TuiPrompt::Choice {
        line: None,
        choices: vec![TuiChoiceRow {
            index: 1,
            id: "help".to_owned(),
            text: "Help.".to_owned(),
            is_available: true,
            unavailable_reason: None,
            is_visible: true,
        }],
        selected: 0,
        mode: PromptMode::Normal,
        input: TextBuffer::default(),
        command: TextBuffer::default(),
        show_help,
    }
}

pub(super) fn condition_help_state(keymap: Keymap) -> TuiState {
    TuiState {
        asset: "asset".to_owned(),
        block: "start".to_owned(),
        transcript: Vec::new(),
        prompt: condition_prompt(true),
        status: "condition".to_owned(),
        key_hints: KeyHints::Contextual,
        keymap,
        ..TuiState::default()
    }
}

pub(super) fn condition_prompt(show_help: bool) -> TuiPrompt {
    TuiPrompt::Condition {
        query: "trusts(mira)".to_owned(),
        selected: true,
        mode: PromptMode::Normal,
        command: TextBuffer::default(),
        show_help,
    }
}

pub(super) fn effect_prompt(show_help: bool) -> TuiPrompt {
    TuiPrompt::Effect {
        mode: "blocking".to_owned(),
        id: "grant#1".to_owned(),
        function: "grant_item".to_owned(),
        args: "(map)".to_owned(),
        input_mode: PromptMode::Insert,
        input: TextBuffer::default(),
        command: TextBuffer::default(),
        show_help,
    }
}

pub(super) fn control_keys(prompt: &TuiPrompt, keymap: Keymap) -> Vec<&'static str> {
    controls::controls_for_prompt(prompt, keymap)
        .into_iter()
        .map(|control| control.keys)
        .collect()
}

pub(super) fn render_tui_content(state: &TuiState, width: u16, height: u16) -> String {
    let backend = ratatui::backend::TestBackend::new(width, height);
    let mut terminal = Terminal::new(backend).expect("terminal");
    let messages = Messages::load(&crate::i18n::UiLocale::default()).expect("messages");

    terminal
        .draw(|frame| render_tui(frame, state, &messages))
        .expect("draw");
    terminal
        .backend()
        .buffer()
        .content
        .iter()
        .map(|cell| cell.symbol())
        .collect::<String>()
}
