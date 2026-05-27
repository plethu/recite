use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

use crate::i18n::{Messages, MsgId};
use crate::tui::{KeyHints, Keymap, PromptMode};

use super::super::state::{TuiPrompt, TuiState, prompt_mode};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) enum ControlAvailability {
    All,
    Standard,
    Vim,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(super) struct TuiControl {
    pub(super) keys: &'static str,
    pub(super) action: MsgId,
    pub(super) description: MsgId,
    pub(super) availability: ControlAvailability,
    pub(super) footer: bool,
    pub(super) compact_footer: bool,
}

impl TuiControl {
    const fn all(
        keys: &'static str,
        action: MsgId,
        description: MsgId,
        footer: bool,
        compact_footer: bool,
    ) -> Self {
        Self {
            keys,
            action,
            description,
            availability: ControlAvailability::All,
            footer,
            compact_footer,
        }
    }

    const fn standard(
        keys: &'static str,
        action: MsgId,
        description: MsgId,
        footer: bool,
        compact_footer: bool,
    ) -> Self {
        Self {
            keys,
            action,
            description,
            availability: ControlAvailability::Standard,
            footer,
            compact_footer,
        }
    }

    const fn vim(
        keys: &'static str,
        action: MsgId,
        description: MsgId,
        footer: bool,
        compact_footer: bool,
    ) -> Self {
        Self {
            keys,
            action,
            description,
            availability: ControlAvailability::Vim,
            footer,
            compact_footer,
        }
    }

    fn is_available(self, keymap: Keymap) -> bool {
        matches!(
            (self.availability, keymap),
            (ControlAvailability::All, _)
                | (ControlAvailability::Standard, Keymap::Standard)
                | (ControlAvailability::Vim, Keymap::Vim)
        )
    }
}

pub(super) fn controls_for_prompt(prompt: &TuiPrompt, keymap: Keymap) -> Vec<TuiControl> {
    let mut controls = Vec::new();
    let help_mode = prompt_mode(prompt) == PromptMode::Help;
    if help_mode {
        controls.push(TuiControl::all(
            "? / Esc",
            MsgId::TuiHelpActionClose,
            MsgId::TuiHelpDescriptionClose,
            true,
            true,
        ));
        controls.push(TuiControl::all(
            "Ctrl-C",
            MsgId::TuiHelpActionQuit,
            MsgId::TuiHelpDescriptionInterrupt,
            false,
            false,
        ));
    }
    match prompt {
        TuiPrompt::Choice { .. } => {
            controls.extend([
                TuiControl::all(
                    "Up/Down",
                    MsgId::TuiHelpActionMove,
                    MsgId::TuiHelpDescriptionMoveChoice,
                    true,
                    true,
                ),
                TuiControl::vim(
                    "j / k",
                    MsgId::TuiHelpActionMove,
                    MsgId::TuiHelpDescriptionMoveChoice,
                    true,
                    false,
                ),
                TuiControl::all(
                    "Enter",
                    MsgId::TuiHelpActionSubmit,
                    MsgId::TuiHelpDescriptionSubmitChoice,
                    true,
                    true,
                ),
                TuiControl::standard(
                    "ID/index",
                    MsgId::TuiHelpActionInput,
                    MsgId::TuiHelpDescriptionInputChoice,
                    true,
                    false,
                ),
                TuiControl::vim(
                    "i",
                    MsgId::TuiHelpActionInput,
                    MsgId::TuiHelpDescriptionInputChoice,
                    true,
                    false,
                ),
            ]);
        }
        TuiPrompt::Condition { .. } => {
            controls.extend([
                TuiControl::all(
                    "Up/Down",
                    MsgId::TuiHelpActionMove,
                    MsgId::TuiHelpDescriptionMoveCondition,
                    true,
                    true,
                ),
                TuiControl::standard(
                    "y / n",
                    MsgId::TuiHelpActionShortcut,
                    MsgId::TuiHelpDescriptionShortcutCondition,
                    true,
                    true,
                ),
                TuiControl::all(
                    "Enter",
                    MsgId::TuiHelpActionSubmit,
                    MsgId::TuiHelpDescriptionSubmitCondition,
                    true,
                    true,
                ),
            ]);
        }
        TuiPrompt::Effect { .. } => controls.push(TuiControl::all(
            "Enter",
            MsgId::TuiHelpActionSubmit,
            MsgId::TuiHelpDescriptionSubmitEffect,
            true,
            true,
        )),
        TuiPrompt::Finished { .. } => controls.push(TuiControl::all(
            "Enter/Esc/q",
            MsgId::TuiHelpActionQuit,
            MsgId::TuiHelpDescriptionFinished,
            true,
            true,
        )),
        TuiPrompt::None => {}
    }
    if !help_mode && !matches!(prompt, TuiPrompt::None) {
        controls.push(TuiControl::all(
            "?",
            MsgId::TuiHelpActionHelp,
            MsgId::TuiHelpDescriptionOpenHelp,
            true,
            true,
        ));
        controls.push(TuiControl::all(
            "Ctrl-C",
            MsgId::TuiHelpActionQuit,
            MsgId::TuiHelpDescriptionInterrupt,
            false,
            false,
        ));
    }
    if keymap == Keymap::Vim {
        controls.extend([
            TuiControl::vim(
                ":q",
                MsgId::TuiHelpActionQuit,
                MsgId::TuiHelpDescriptionQuit,
                true,
                false,
            ),
            TuiControl::vim(
                ":",
                MsgId::TuiHelpActionCommand,
                MsgId::TuiHelpDescriptionCommand,
                true,
                false,
            ),
        ]);
    }
    controls
        .into_iter()
        .filter(|control| control.is_available(keymap))
        .collect()
}

pub(super) fn render_footer<'a>(state: &'a TuiState, messages: &'a Messages) -> Line<'a> {
    let mut help = match state.key_hints {
        KeyHints::Hidden => String::new(),
        KeyHints::Compact => compact_footer_controls(&state.prompt, state.keymap, messages),
        KeyHints::Contextual => contextual_footer_controls(&state.prompt, state.keymap, messages),
    };
    if !state.deferred_queue.is_empty() && state.key_hints != KeyHints::Hidden {
        let queue = if state.key_hints == KeyHints::Compact {
            "Ctrl-D".to_owned()
        } else {
            format!(
                "Ctrl-D {}",
                if state.deferred_queue_expanded {
                    messages.text(MsgId::TuiHelpActionClose)
                } else {
                    messages.text(MsgId::TuiHelpActionQueue)
                }
            )
        };
        if help.is_empty() {
            help = queue;
        } else {
            help.push_str(" | ");
            help.push_str(&queue);
        }
    }
    if help.is_empty() {
        return Line::from(Span::styled(
            state.status.as_str(),
            Style::default().fg(Color::DarkGray),
        ));
    }
    if state.status.is_empty() {
        return Line::from(help);
    }
    Line::from(vec![
        Span::styled(state.status.as_str(), Style::default().fg(Color::DarkGray)),
        Span::raw("  "),
        Span::raw(help),
    ])
}

fn compact_footer_controls(prompt: &TuiPrompt, keymap: Keymap, messages: &Messages) -> String {
    footer_controls(prompt, keymap, messages, true)
}

fn contextual_footer_controls(prompt: &TuiPrompt, keymap: Keymap, messages: &Messages) -> String {
    if prompt_mode(prompt) == PromptMode::Command {
        return messages.text(MsgId::TuiFooterCommand);
    }
    footer_controls(prompt, keymap, messages, false)
}

pub(super) fn help_footer_control(
    prompt: &TuiPrompt,
    keymap: Keymap,
    messages: &Messages,
) -> String {
    controls_for_prompt(prompt, keymap)
        .into_iter()
        .find(|control| control.action == MsgId::TuiHelpActionClose)
        .map(|control| footer_control_text(control, messages, false))
        .unwrap_or_default()
}

fn footer_controls(
    prompt: &TuiPrompt,
    keymap: Keymap,
    messages: &Messages,
    compact: bool,
) -> String {
    controls_for_prompt(prompt, keymap)
        .into_iter()
        .filter(|control| {
            if compact {
                control.compact_footer
            } else {
                control.footer
            }
        })
        .map(|control| footer_control_text(control, messages, compact))
        .collect::<Vec<_>>()
        .join(" | ")
}

fn footer_control_text(control: TuiControl, messages: &Messages, compact: bool) -> String {
    if compact {
        return control.keys.to_owned();
    }
    format!("{} {}", control.keys, messages.text(control.action))
}
