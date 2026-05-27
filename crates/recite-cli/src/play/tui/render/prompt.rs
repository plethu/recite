use ratatui::{
    style::{Color, Modifier, Style},
    text::{Line, Span},
};

use crate::i18n::{Messages, MsgId};
use crate::tui::{Keymap, TextBuffer};

use super::super::state::{
    TuiDeferredEffectRow, TuiDeferredQueueState, TuiPrompt, TuiState, TuiTranscriptKind,
};
use super::transcript::{prompt_header_line, wrap_continuation};

pub(super) fn render_prompt<'a>(
    prompt: &'a TuiPrompt,
    keymap: Keymap,
    width: u16,
    messages: &'a Messages,
) -> Vec<Line<'a>> {
    match prompt {
        TuiPrompt::None => vec![Line::from(Span::styled(
            messages.text(MsgId::TuiWaiting),
            Style::default().fg(Color::DarkGray),
        ))],
        TuiPrompt::Finished { .. } => vec![Line::from("")],
        TuiPrompt::Condition {
            query,
            command,
            selected,
            ..
        } => {
            vec![
                prompt_header_line(TuiTranscriptKind::Condition, Some(query.as_str()), messages),
                condition_row(*selected, true, keymap, messages),
                condition_row(!*selected, false, keymap, messages),
                command_line(command),
            ]
        }
        TuiPrompt::Effect {
            mode,
            id,
            function,
            args,
            input,
            command,
            ..
        } => {
            vec![
                prompt_header_line(TuiTranscriptKind::Effect, Some(id.as_str()), messages),
                metadata_line(messages.text(MsgId::TuiMetadataMode), mode),
                metadata_line(messages.text(MsgId::TuiMetadataRuntimeEffectId), id),
                metadata_line(messages.text(MsgId::TuiMetadataFunction), function),
                metadata_line(messages.text(MsgId::TuiMetadataArgs), args),
                if command.is_empty() {
                    Line::from(Span::styled(
                        messages.text(MsgId::TuiAckEnterHint),
                        Style::default()
                            .fg(Color::Magenta)
                            .add_modifier(Modifier::BOLD),
                    ))
                } else {
                    input_line(messages.text(MsgId::TuiInputAck), input, command)
                },
            ]
        }
        TuiPrompt::Choice {
            line,
            choices,
            selected,
            command,
            ..
        } => {
            let mut lines = Vec::new();
            if let Some(line) = line {
                lines.push(prompt_header_line(
                    TuiTranscriptKind::Prompt,
                    Some(line.id.as_str()),
                    messages,
                ));
                lines.extend(wrap_continuation(line.text.as_str(), width as usize, 2));
            } else {
                lines.push(prompt_header_line(
                    TuiTranscriptKind::Prompt,
                    None,
                    messages,
                ));
            }
            let selected_index = choices.get(*selected).map(|choice| choice.index);
            for choice in choices.iter().filter(|choice| choice.is_visible) {
                let style = if choice.is_available {
                    Style::default()
                } else {
                    Style::default().fg(Color::DarkGray)
                };
                let is_selected = Some(choice.index) == selected_index;
                let marker = if is_selected { ">" } else { " " };
                let choice_chrome_style = choice_chrome_style(is_selected, choice.is_available);
                let suffix = choice
                    .unavailable_reason
                    .as_deref()
                    .filter(|reason| !reason.is_empty())
                    .map(|reason| {
                        messages.format(
                            MsgId::TuiChoiceUnavailableReason,
                            [("reason", reason.to_owned())],
                        )
                    })
                    .unwrap_or_else(|| {
                        if choice.is_available {
                            String::new()
                        } else {
                            messages.text(MsgId::TuiChoiceUnavailable)
                        }
                    });
                lines.push(Line::from(vec![
                    Span::styled(marker, choice_chrome_style),
                    Span::raw(" "),
                    Span::styled(format!("{:>2}", choice.index), choice_chrome_style),
                    Span::raw("  "),
                    Span::styled(format!("{:<16}", choice.id), choice_chrome_style),
                    Span::styled(choice.text.as_str(), style),
                    Span::styled(suffix, Style::default().fg(Color::DarkGray)),
                ]));
            }
            lines.push(command_line(command));
            lines
        }
    }
}

pub(super) fn render_deferred_queue<'a>(
    state: &'a TuiState,
    messages: &'a Messages,
) -> Vec<Line<'a>> {
    let status = match state.deferred_queue_state {
        Some(TuiDeferredQueueState::Ready) => messages.text(MsgId::TuiDeferredQueueReadyAtEnd),
        Some(TuiDeferredQueueState::Scheduled) | None => {
            messages.text(MsgId::TuiDeferredQueueScheduled)
        }
    };
    let mut lines = vec![Line::from(vec![
        Span::styled(
            messages.text(MsgId::TuiDeferredQueueTitle),
            Style::default()
                .fg(Color::Magenta)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw(" "),
        Span::styled(status, Style::default().fg(Color::DarkGray)),
    ])];
    lines.extend(state.deferred_queue.iter().map(deferred_queue_row).take(5));
    lines
}

fn choice_chrome_style(is_selected: bool, is_available: bool) -> Style {
    let style = if is_available {
        Style::default().fg(Color::Cyan)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    if is_selected {
        style.add_modifier(Modifier::BOLD)
    } else {
        style
    }
}

fn deferred_queue_row(effect: &TuiDeferredEffectRow) -> Line<'_> {
    Line::from(vec![
        Span::styled("  ", Style::default().fg(Color::DarkGray)),
        Span::styled(effect.id.as_str(), Style::default().fg(Color::DarkGray)),
        Span::raw(" "),
        Span::raw(effect.function.as_str()),
        Span::raw(" "),
        Span::raw(effect.args.as_str()),
    ])
}

fn condition_row<'a>(
    is_selected: bool,
    value: bool,
    keymap: Keymap,
    messages: &'a Messages,
) -> Line<'a> {
    let marker_style = if is_selected {
        Style::default()
            .fg(Color::Yellow)
            .add_modifier(Modifier::BOLD)
    } else {
        Style::default().fg(Color::DarkGray)
    };
    let label = match (keymap, value) {
        (Keymap::Standard, true) => messages.text(MsgId::TuiConditionYesShortcutRow),
        (Keymap::Standard, false) => messages.text(MsgId::TuiConditionNoShortcutRow),
        (_, true) => messages.text(MsgId::TuiConditionYesRow),
        (_, false) => messages.text(MsgId::TuiConditionNoRow),
    };
    Line::from(vec![
        Span::styled(if is_selected { ">" } else { " " }, marker_style),
        Span::raw(" "),
        Span::raw(label),
    ])
}

fn command_line<'a>(command: &'a TextBuffer) -> Line<'a> {
    if command.is_empty() {
        return Line::from("");
    }
    Line::from(vec![
        Span::styled(":", Style::default().fg(Color::DarkGray)),
        Span::raw(command.as_str()),
    ])
}

fn input_line<'a>(label: String, input: &'a TextBuffer, command: &'a TextBuffer) -> Line<'a> {
    if !command.is_empty() {
        return Line::from(vec![
            Span::styled(":", Style::default().fg(Color::DarkGray)),
            Span::raw(command.as_str()),
        ]);
    }
    Line::from(vec![
        Span::styled(format!("{label:<8}"), Style::default().fg(Color::DarkGray)),
        Span::raw(input.as_str()),
    ])
}

fn metadata_line<'a>(label: String, value: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{label:<18}"), Style::default().fg(Color::DarkGray)),
        Span::raw(value),
    ])
}
