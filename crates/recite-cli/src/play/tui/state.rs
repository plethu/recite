use crate::tui::{
    KeyHints, Keymap, PromptMode, TextBuffer, TuiIntent, TuiInteractionState, TuiPalette,
};

#[derive(Default)]
pub(super) struct TuiState {
    pub(super) asset: String,
    pub(super) block: String,
    pub(super) transcript: Vec<TuiTranscriptEntry>,
    pub(super) deferred_queue: Vec<TuiDeferredEffectRow>,
    pub(super) deferred_queue_state: Option<TuiDeferredQueueState>,
    pub(super) deferred_queue_expanded: bool,
    pub(super) prompt: TuiPrompt,
    pub(super) status: String,
    pub(super) key_hints: KeyHints,
    pub(super) keymap: Keymap,
    pub(super) palette: TuiPalette,
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

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum TuiDeferredQueueState {
    Scheduled,
    Ready,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub(super) struct TuiDeferredEffectRow {
    pub(super) id: String,
    pub(super) function: String,
    pub(super) args: String,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub(super) enum TuiPrompt {
    #[default]
    None,
    Choice {
        line: Option<TuiPromptLine>,
        choices: Vec<TuiChoiceRow>,
        selected: usize,
        interaction: TuiInteractionState,
        input: TextBuffer,
    },
    Condition {
        query: String,
        selected: bool,
        interaction: TuiInteractionState,
    },
    EnumCondition {
        query: String,
        interaction: TuiInteractionState,
        input: TextBuffer,
    },
    Effect {
        mode: String,
        id: String,
        function: String,
        args: String,
        interaction: TuiInteractionState,
        input: TextBuffer,
    },
    Finished {
        interaction: TuiInteractionState,
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

pub(super) fn initial_interaction(keymap: Keymap) -> TuiInteractionState {
    TuiInteractionState::new(initial_prompt_mode(keymap)).with_help(false)
}

pub(super) fn finished_interaction() -> TuiInteractionState {
    TuiInteractionState::new(PromptMode::Finished).with_help(false)
}

pub(super) fn initial_choice_selection(choices: &[TuiChoiceRow]) -> usize {
    choices
        .iter()
        .position(|choice| choice.is_visible && choice.is_available)
        .or_else(|| choices.iter().position(|choice| choice.is_visible))
        .unwrap_or(0)
}

pub(super) fn prompt_mode(prompt: &TuiPrompt) -> PromptMode {
    prompt_interaction(prompt)
        .map(TuiInteractionState::effective_mode)
        .unwrap_or(PromptMode::Normal)
}

pub(super) fn set_prompt_mode(prompt: &mut TuiPrompt, mode: PromptMode) {
    if let Some(interaction) = prompt_interaction_mut(prompt) {
        interaction.set_mode(mode);
    }
}

pub(super) fn toggle_help(prompt: &mut TuiPrompt) {
    if let Some(interaction) = prompt_interaction_mut(prompt) {
        interaction.toggle_help();
    }
}

pub(super) fn close_help(prompt: &mut TuiPrompt) {
    if let Some(interaction) = prompt_interaction_mut(prompt) {
        interaction.close_help();
    }
}

pub(super) fn toggle_deferred_queue(state: &mut TuiState) {
    if !state.deferred_queue.is_empty() {
        state.deferred_queue_expanded = !state.deferred_queue_expanded;
    }
}

pub(super) fn start_prompt_command(prompt: &mut TuiPrompt) {
    if let Some(interaction) = prompt_interaction_mut(prompt) {
        interaction.start_command();
    }
}

pub(super) fn prompt_command(prompt: &TuiPrompt) -> &str {
    prompt_interaction(prompt)
        .map(TuiInteractionState::command)
        .unwrap_or("")
}

pub(super) fn mutate_prompt_command(prompt: &mut TuiPrompt, intent: TuiIntent) {
    if let Some(interaction) = prompt_interaction_mut(prompt) {
        interaction.mutate_command(intent);
    }
}

pub(super) fn prompt_input(prompt: &TuiPrompt) -> &str {
    match prompt {
        TuiPrompt::Choice { input, .. }
        | TuiPrompt::EnumCondition { input, .. }
        | TuiPrompt::Effect { input, .. } => input.as_str(),
        TuiPrompt::None | TuiPrompt::Condition { .. } | TuiPrompt::Finished { .. } => "",
    }
}

pub(super) fn mutate_prompt_input(prompt: &mut TuiPrompt, intent: TuiIntent) {
    match prompt {
        TuiPrompt::Choice {
            input, interaction, ..
        } => {
            if matches!(intent, TuiIntent::Text(_)) {
                interaction.set_mode(PromptMode::Insert);
            }
            input.apply_intent(intent);
        }
        TuiPrompt::EnumCondition {
            input, interaction, ..
        } => {
            if matches!(intent, TuiIntent::Text(_)) {
                interaction.set_mode(PromptMode::Insert);
            }
            input.apply_intent(intent);
        }
        TuiPrompt::Effect { input, .. } => input.apply_intent(intent),
        TuiPrompt::None | TuiPrompt::Condition { .. } | TuiPrompt::Finished { .. } => {}
    }
}

pub(super) fn selected_choice_id(prompt: &TuiPrompt) -> Option<&str> {
    match prompt {
        TuiPrompt::Choice {
            choices, selected, ..
        } => choices.get(*selected).map(|choice| choice.id.as_str()),
        TuiPrompt::None
        | TuiPrompt::Condition { .. }
        | TuiPrompt::EnumCondition { .. }
        | TuiPrompt::Effect { .. }
        | TuiPrompt::Finished { .. } => None,
    }
}

pub(super) fn move_choice_selection(prompt: &mut TuiPrompt, direction: isize) {
    let (choices, selected) = match prompt {
        TuiPrompt::Choice {
            choices, selected, ..
        } => (choices, selected),
        TuiPrompt::None
        | TuiPrompt::Condition { .. }
        | TuiPrompt::EnumCondition { .. }
        | TuiPrompt::Effect { .. }
        | TuiPrompt::Finished { .. } => return,
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
        TuiPrompt::None
        | TuiPrompt::Choice { .. }
        | TuiPrompt::EnumCondition { .. }
        | TuiPrompt::Effect { .. }
        | TuiPrompt::Finished { .. } => None,
    }
}

pub(super) fn move_condition_selection(prompt: &mut TuiPrompt) {
    match prompt {
        TuiPrompt::Condition { selected, .. } => *selected = !*selected,
        TuiPrompt::None
        | TuiPrompt::Choice { .. }
        | TuiPrompt::EnumCondition { .. }
        | TuiPrompt::Effect { .. }
        | TuiPrompt::Finished { .. } => {}
    }
}

pub(super) fn set_condition_selection(prompt: &mut TuiPrompt, value: bool) {
    match prompt {
        TuiPrompt::Condition { selected, .. } => *selected = value,
        TuiPrompt::None
        | TuiPrompt::Choice { .. }
        | TuiPrompt::EnumCondition { .. }
        | TuiPrompt::Effect { .. }
        | TuiPrompt::Finished { .. } => {}
    }
}

pub(super) fn prompt_label(label: String) -> String {
    format!("{label} ")
}

fn prompt_interaction(prompt: &TuiPrompt) -> Option<&TuiInteractionState> {
    match prompt {
        TuiPrompt::Choice { interaction, .. }
        | TuiPrompt::Condition { interaction, .. }
        | TuiPrompt::EnumCondition { interaction, .. }
        | TuiPrompt::Effect { interaction, .. }
        | TuiPrompt::Finished { interaction } => Some(interaction),
        TuiPrompt::None => None,
    }
}

fn prompt_interaction_mut(prompt: &mut TuiPrompt) -> Option<&mut TuiInteractionState> {
    match prompt {
        TuiPrompt::Choice { interaction, .. }
        | TuiPrompt::Condition { interaction, .. }
        | TuiPrompt::EnumCondition { interaction, .. }
        | TuiPrompt::Effect { interaction, .. }
        | TuiPrompt::Finished { interaction } => Some(interaction),
        TuiPrompt::None => None,
    }
}

#[cfg(test)]
mod tests;
