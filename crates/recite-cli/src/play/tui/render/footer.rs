use ratatui::{
    style::{Color, Style},
    text::{Line, Span},
};

use crate::i18n::{Messages, MsgId};
use crate::tui::{KeyHints, Keymap, PromptMode};

use super::super::state::{TuiPrompt, TuiState, prompt_mode};
use super::controls::{TuiControl, controls_for_prompt};

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

fn compact_footer_controls(prompt: &TuiPrompt, keymap: Keymap, messages: &Messages) -> String {
    footer_controls(prompt, keymap, messages, true)
}

fn contextual_footer_controls(prompt: &TuiPrompt, keymap: Keymap, messages: &Messages) -> String {
    if prompt_mode(prompt) == PromptMode::Command {
        return messages.text(MsgId::TuiFooterCommand);
    }
    footer_controls(prompt, keymap, messages, false)
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
