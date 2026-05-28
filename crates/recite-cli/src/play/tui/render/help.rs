use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use crate::i18n::{Messages, MsgId};

use super::super::state::TuiState;
use super::controls::controls_for_prompt;

pub(super) fn help_overlay_lines(state: &TuiState, messages: &Messages) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(Span::styled(
            messages.text(MsgId::TuiHelpTitle),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        )),
        Line::from(""),
        help_table_row(
            messages.text(MsgId::TuiHelpKeyHeading),
            messages.text(MsgId::TuiHelpActionHeading),
            messages.text(MsgId::TuiHelpDescriptionHeading),
            true,
        ),
    ];
    if !state.deferred_queue.is_empty() {
        lines.push(help_table_row(
            "Ctrl-D",
            messages.text(MsgId::TuiHelpActionQueue),
            messages.text(MsgId::TuiHelpDescriptionQueue),
            false,
        ));
    }
    for control in controls_for_prompt(&state.prompt, state.keymap) {
        lines.push(help_table_row(
            control.keys,
            messages.text(control.action),
            messages.text(control.description),
            false,
        ));
    }
    lines
}

fn help_table_row(
    key: impl Into<String>,
    action: impl Into<String>,
    description: impl Into<String>,
    heading: bool,
) -> Line<'static> {
    let style = if heading {
        Style::default()
            .fg(Color::DarkGray)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default()
    };
    vec![
        Span::styled(format!("{:<12}", key.into()), style),
        Span::styled(format!("{:<14}", action.into()), style),
        Span::styled(description.into(), style),
    ]
    .into()
}
