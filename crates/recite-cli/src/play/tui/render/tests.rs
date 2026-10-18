use ratatui::style::Color;

use crate::i18n::Messages;
use crate::tui::{
    KeyHints, Keymap, PromptMode, TextBuffer, TuiContrast, TuiInteractionState, TuiPalette,
};

use super::super::state::{
    TuiChoiceRow, TuiDeferredEffectRow, TuiDeferredQueueState, TuiPrompt, TuiPromptLine, TuiState,
    TuiTranscriptEntry, TuiTranscriptKind,
};
use super::*;

#[path = "tests/support.rs"]
mod support;
use support::*;

#[path = "tests/controls/tests.rs"]
mod control_cases;
#[path = "tests/palette/tests.rs"]
mod palette;
#[path = "tests/transcript/tests.rs"]
mod transcript_cases;
