mod config;
mod input;
mod interaction;
mod style;
mod terminal;

pub(crate) use config::{KeyHints, Keymap, TuiContrast, TuiSettings};
pub(crate) use input::{PromptMode, TextBuffer, TuiIntent, map_key};
pub(crate) use interaction::{GlobalAction, TuiInteractionState, command_quits, global_action};
pub(crate) use style::TuiPalette;
pub(crate) use terminal::{enter_terminal, restore_terminal};

#[cfg(test)]
mod tests;
