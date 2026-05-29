use super::{PromptMode, TextBuffer, TuiIntent};

#[derive(Clone, Debug, Eq, PartialEq)]
pub(crate) struct TuiInteractionState {
    mode: PromptMode,
    command: TextBuffer,
    show_help: bool,
}

impl TuiInteractionState {
    pub(crate) fn new(mode: PromptMode) -> Self {
        Self {
            mode,
            command: TextBuffer::default(),
            show_help: false,
        }
    }

    pub(crate) fn with_help(mut self, show_help: bool) -> Self {
        self.show_help = show_help;
        self
    }

    pub(crate) fn effective_mode(&self) -> PromptMode {
        if self.show_help {
            PromptMode::Help
        } else {
            self.mode
        }
    }

    pub(crate) fn set_mode(&mut self, mode: PromptMode) {
        self.mode = mode;
        self.show_help = false;
    }

    pub(crate) fn toggle_help(&mut self) {
        self.show_help = !self.show_help;
    }

    pub(crate) fn close_help(&mut self) {
        self.show_help = false;
    }

    pub(crate) fn start_command(&mut self) {
        self.mode = PromptMode::Command;
        self.command.clear();
        self.show_help = false;
    }

    pub(crate) fn command(&self) -> &str {
        self.command.as_str()
    }

    pub(crate) fn mutate_command(&mut self, intent: TuiIntent) {
        self.command.apply_intent(intent);
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum GlobalAction {
    Quit,
    OpenCommand,
    ToggleHelp,
    CloseHelp,
    ToggleAuxiliaryPanel,
}

pub(crate) fn global_action(mode: PromptMode, intent: TuiIntent) -> Option<GlobalAction> {
    match intent {
        TuiIntent::Quit => Some(GlobalAction::Quit),
        TuiIntent::OpenCommand => Some(GlobalAction::OpenCommand),
        TuiIntent::ToggleHelp => Some(GlobalAction::ToggleHelp),
        TuiIntent::ToggleAuxiliaryPanel => Some(GlobalAction::ToggleAuxiliaryPanel),
        TuiIntent::Cancel if mode == PromptMode::Help => Some(GlobalAction::CloseHelp),
        _ => None,
    }
}

pub(crate) fn command_quits(command: &str) -> bool {
    matches!(command.trim(), "q" | "quit")
}
