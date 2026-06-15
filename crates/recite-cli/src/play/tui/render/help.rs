use ratatui::{
    style::Modifier,
    text::{Line, Span},
};

use crate::i18n::{Messages, MsgId};
use crate::tui::TuiPalette;

use super::super::state::TuiState;
use super::controls::controls_for_prompt;

pub(super) fn help_overlay_lines(state: &TuiState, messages: &Messages) -> Vec<Line<'static>> {
    let mut lines = vec![
        Line::from(Span::styled(
            messages.text(MsgId::TuiHelpTitle),
            state.palette.title(),
        )),
        Line::from(""),
        help_table_row(
            messages.text(MsgId::TuiHelpKeyHeading),
            messages.text(MsgId::TuiHelpActionHeading),
            messages.text(MsgId::TuiHelpDescriptionHeading),
            true,
            state.palette,
        ),
    ];
    if !state.deferred_queue.is_empty() {
        lines.push(help_table_row(
            "Ctrl-D",
            messages.text(MsgId::TuiHelpActionQueue),
            messages.text(MsgId::TuiHelpDescriptionQueue),
            false,
            state.palette,
        ));
    }
    for control in controls_for_prompt(&state.prompt, state.keymap) {
        lines.push(help_table_row(
            control.keys,
            messages.text(control.action),
            messages.text(control.description),
            false,
            state.palette,
        ));
    }
    lines
}

fn help_table_row(
    key: impl Into<String>,
    action: impl Into<String>,
    description: impl Into<String>,
    heading: bool,
    palette: TuiPalette,
) -> Line<'static> {
    let style = if heading {
        palette.muted().add_modifier(Modifier::BOLD)
    } else {
        palette.plain()
    };
    vec![
        Span::styled(format!("{:<12}", key.into()), style),
        Span::styled(format!("{:<14}", action.into()), style),
        Span::styled(description.into(), style),
    ]
    .into()
}
