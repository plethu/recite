use crate::tui::{KeyHints, Keymap, PromptMode, TextBuffer, TuiIntent};

#[derive(Default)]
pub(super) struct TuiState {
    pub(super) asset: String,
    pub(super) block: String,
    pub(super) transcript: Vec<TuiTranscriptEntry>,
    pub(super) prompt: TuiPrompt,
    pub(super) status: String,
    pub(super) key_hints: KeyHints,
    pub(super) keymap: Keymap,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct TuiTranscriptEntry {
    pub(super) kind: TuiTranscriptKind,
    pub(super) id: Option<String>,
    pub(super) text: String,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TuiTranscriptKind {
    Line,
    Prompt,
    Choice,
    Condition,
    Effect,
    Ack,
    Deferred,
    End,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) enum TuiPrompt {
    #[default]
    None,
    Choice {
        line: Option<TuiPromptLine>,
        choices: Vec<TuiChoiceRow>,
        selected: usize,
        mode: PromptMode,
        input: TextBuffer,
        command: TextBuffer,
        show_help: bool,
    },
    Condition {
        query: String,
        selected: bool,
        mode: PromptMode,
        command: TextBuffer,
        show_help: bool,
    },
    Effect {
        mode: String,
        id: String,
        function: String,
        args: String,
        input_mode: PromptMode,
        input: TextBuffer,
        command: TextBuffer,
        show_help: bool,
    },
    Finished {
        show_help: bool,
    },
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct TuiPromptLine {
    pub(super) id: String,
    pub(super) text: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct TuiChoiceRow {
    pub(super) index: usize,
    pub(super) id: String,
    pub(super) text: String,
    pub(super) is_available: bool,
    pub(super) unavailable_reason: Option<String>,
    pub(super) is_visible: bool,
}

pub(super) fn initial_prompt_mode(keymap: Keymap) -> PromptMode {
    match keymap {
        Keymap::Standard => PromptMode::Insert,
        Keymap::Vim => PromptMode::Normal,
    }
}

pub(super) fn initial_choice_selection(choices: &[TuiChoiceRow]) -> usize {
    choices
        .iter()
        .position(|choice| choice.is_visible && choice.is_available)
        .or_else(|| choices.iter().position(|choice| choice.is_visible))
        .unwrap_or(0)
}

pub(super) fn prompt_mode(prompt: &TuiPrompt) -> PromptMode {
    match prompt {
        TuiPrompt::Choice {
            mode, show_help, ..
        }
        | TuiPrompt::Condition {
            mode, show_help, ..
        } => {
            if *show_help {
                PromptMode::Help
            } else {
                *mode
            }
        }
        TuiPrompt::Effect {
            input_mode,
            show_help,
            ..
        } => {
            if *show_help {
                PromptMode::Help
            } else {
                *input_mode
            }
        }
        TuiPrompt::Finished { show_help } => {
            if *show_help {
                PromptMode::Help
            } else {
                PromptMode::Normal
            }
        }
        _ => PromptMode::Normal,
    }
}

pub(super) fn set_prompt_mode(prompt: &mut TuiPrompt, mode: PromptMode) {
    match prompt {
        TuiPrompt::Choice {
            mode: prompt_mode,
            show_help,
            ..
        }
        | TuiPrompt::Condition {
            mode: prompt_mode,
            show_help,
            ..
        } => {
            *prompt_mode = mode;
            *show_help = false;
        }
        TuiPrompt::Effect {
            input_mode,
            show_help,
            ..
        } => {
            *input_mode = mode;
            *show_help = false;
        }
        _ => {}
    }
}

pub(super) fn toggle_help(prompt: &mut TuiPrompt) {
    match prompt {
        TuiPrompt::Choice { show_help, .. }
        | TuiPrompt::Condition { show_help, .. }
        | TuiPrompt::Effect { show_help, .. }
        | TuiPrompt::Finished { show_help } => *show_help = !*show_help,
        _ => {}
    }
}

pub(super) fn close_help(prompt: &mut TuiPrompt) {
    match prompt {
        TuiPrompt::Choice { show_help, .. }
        | TuiPrompt::Condition { show_help, .. }
        | TuiPrompt::Effect { show_help, .. }
        | TuiPrompt::Finished { show_help } => *show_help = false,
        _ => {}
    }
}

pub(super) fn set_command(prompt: &mut TuiPrompt, command: TextBuffer) {
    match prompt {
        TuiPrompt::Choice {
            command: prompt_command,
            ..
        }
        | TuiPrompt::Condition {
            command: prompt_command,
            ..
        }
        | TuiPrompt::Effect {
            command: prompt_command,
            ..
        } => *prompt_command = command,
        _ => {}
    }
}

pub(super) fn prompt_command(prompt: &TuiPrompt) -> &str {
    match prompt {
        TuiPrompt::Choice { command, .. }
        | TuiPrompt::Condition { command, .. }
        | TuiPrompt::Effect { command, .. } => command.as_str(),
        _ => "",
    }
}

pub(super) fn mutate_prompt_command(prompt: &mut TuiPrompt, intent: TuiIntent) {
    match prompt {
        TuiPrompt::Choice { command, .. }
        | TuiPrompt::Condition { command, .. }
        | TuiPrompt::Effect { command, .. } => mutate_buffer(command, intent),
        _ => {}
    }
}

pub(super) fn prompt_input(prompt: &TuiPrompt) -> &str {
    match prompt {
        TuiPrompt::Choice { input, .. } | TuiPrompt::Effect { input, .. } => input.as_str(),
        _ => "",
    }
}

pub(super) fn mutate_prompt_input(prompt: &mut TuiPrompt, intent: TuiIntent) {
    match prompt {
        TuiPrompt::Choice { input, mode, .. } => {
            if matches!(intent, TuiIntent::Text(_)) {
                *mode = PromptMode::Insert;
            }
            mutate_buffer(input, intent);
        }
        TuiPrompt::Effect { input, .. } => mutate_buffer(input, intent),
        TuiPrompt::Condition { .. } => {}
        _ => {}
    }
}

fn mutate_buffer(buffer: &mut TextBuffer, intent: TuiIntent) {
    match intent {
        TuiIntent::Text(ch) => buffer.insert(ch),
        TuiIntent::Backspace => buffer.backspace(),
        TuiIntent::Delete => buffer.delete(),
        TuiIntent::MoveCursorLeft => buffer.move_left(),
        TuiIntent::MoveCursorRight => buffer.move_right(),
        TuiIntent::MoveCursorStart => buffer.move_start(),
        TuiIntent::MoveCursorEnd => buffer.move_end(),
        TuiIntent::ClearLine => buffer.clear(),
        TuiIntent::DeleteWord => buffer.delete_word_before_cursor(),
        _ => {}
    }
}

pub(super) fn selected_choice_id(prompt: &TuiPrompt) -> Option<&str> {
    match prompt {
        TuiPrompt::Choice {
            choices, selected, ..
        } => choices.get(*selected).map(|choice| choice.id.as_str()),
        _ => None,
    }
}

pub(super) fn move_choice_selection(prompt: &mut TuiPrompt, direction: isize) {
    let TuiPrompt::Choice {
        choices, selected, ..
    } = prompt
    else {
        return;
    };
    if choices.is_empty() {
        return;
    }
    let len = choices.len();
    let mut next = *selected;
    for _ in 0..len {
        next = if direction > 0 {
            (next + 1) % len
        } else {
            (next + len - 1) % len
        };
        if choices[next].is_visible && choices[next].is_available {
            *selected = next;
            return;
        }
    }
}

pub(super) fn condition_selection(prompt: &TuiPrompt) -> Option<bool> {
    match prompt {
        TuiPrompt::Condition { selected, .. } => Some(*selected),
        _ => None,
    }
}

pub(super) fn move_condition_selection(prompt: &mut TuiPrompt) {
    if let TuiPrompt::Condition { selected, .. } = prompt {
        *selected = !*selected;
    }
}

pub(super) fn set_condition_selection(prompt: &mut TuiPrompt, value: bool) {
    if let TuiPrompt::Condition { selected, .. } = prompt {
        *selected = value;
    }
}

pub(super) fn prompt_label(label: String) -> String {
    format!("{label} ")
}

#[cfg(test)]
#[path = "state_tests.rs"]
mod tests;
