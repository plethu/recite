use ratatui::text::{Line, Span};

use crate::i18n::{Messages, MsgId};
use crate::tui::{Keymap, TextBuffer, TuiPalette};

use super::super::state::{
    TuiDeferredEffectRow, TuiDeferredQueueState, TuiPrompt, TuiState, TuiTranscriptKind,
};
use super::transcript::{prompt_header_line, wrap_continuation};

pub(super) fn render_prompt<'a>(
    prompt: &'a TuiPrompt,
    keymap: Keymap,
    width: u16,
    messages: &'a Messages,
    palette: TuiPalette,
) -> Vec<Line<'a>> {
    match prompt {
        TuiPrompt::None => vec![Line::from(Span::styled(
            messages.text(MsgId::TuiWaiting),
            palette.muted(),
        ))],
        TuiPrompt::Finished { .. } => vec![Line::from("")],
        TuiPrompt::Condition {
            query,
            interaction,
            selected,
            ..
        } => {
            vec![
                prompt_header_line(
                    TuiTranscriptKind::Condition,
                    Some(query.as_str()),
                    messages,
                    palette,
                ),
                condition_row(*selected, true, keymap, messages, palette),
                condition_row(!*selected, false, keymap, messages, palette),
                command_line(interaction.command(), palette),
            ]
        }
        TuiPrompt::EnumCondition {
            query,
            input,
            interaction,
            ..
        } => {
            vec![
                prompt_header_line(
                    TuiTranscriptKind::Condition,
                    Some(query.as_str()),
                    messages,
                    palette,
                ),
                Line::from(Span::styled(
                    messages.text(MsgId::TuiEnumConditionHint),
                    palette.muted(),
                )),
                input_line(
                    messages.text(MsgId::TuiInputEnumVariant),
                    input,
                    interaction.command(),
                    palette,
                ),
            ]
        }
        TuiPrompt::Effect {
            mode,
            id,
            function,
            args,
            input,
            interaction,
            ..
        } => {
            vec![
                prompt_header_line(
                    TuiTranscriptKind::Effect,
                    Some(id.as_str()),
                    messages,
                    palette,
                ),
                metadata_line(messages.text(MsgId::TuiMetadataMode), mode, palette),
                metadata_line(
                    messages.text(MsgId::TuiMetadataRuntimeEffectId),
                    id,
                    palette,
                ),
                metadata_line(messages.text(MsgId::TuiMetadataFunction), function, palette),
                metadata_line(messages.text(MsgId::TuiMetadataArgs), args, palette),
                if interaction.command().is_empty() {
                    Line::from(Span::styled(
                        messages.text(MsgId::TuiAckEnterHint),
                        palette.emphasis(),
                    ))
                } else {
                    input_line(
                        messages.text(MsgId::TuiInputAck),
                        input,
                        interaction.command(),
                        palette,
                    )
                },
            ]
        }
        TuiPrompt::Choice {
            line,
            choices,
            selected,
            interaction,
            ..
        } => {
            let mut lines = Vec::new();
            if let Some(line) = line {
                lines.push(prompt_header_line(
                    TuiTranscriptKind::Prompt,
                    Some(line.id.as_str()),
                    messages,
                    palette,
                ));
                lines.extend(wrap_continuation(line.text.as_str(), width as usize, 2));
            } else {
                lines.push(prompt_header_line(
                    TuiTranscriptKind::Prompt,
                    None,
                    messages,
                    palette,
                ));
            }
            let selected_index = choices.get(*selected).map(|choice| choice.index);
            for choice in choices.iter().filter(|choice| choice.is_visible) {
                let style = if choice.is_available {
                    palette.plain()
                } else {
                    palette.muted()
                };
                let is_selected = Some(choice.index) == selected_index;
                let marker = if is_selected { ">" } else { " " };
                let choice_chrome_style = palette.choice_chrome(is_selected, choice.is_available);
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
                    Span::styled(suffix, palette.muted()),
                ]));
            }
            lines.push(command_line(interaction.command(), palette));
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
            state.palette.emphasis(),
        ),
        Span::raw(" "),
        Span::styled(status, state.palette.muted()),
    ])];
    lines.extend(
        state
            .deferred_queue
            .iter()
            .map(|effect| deferred_queue_row(effect, state.palette))
            .take(5),
    );
    lines
}

fn deferred_queue_row(effect: &TuiDeferredEffectRow, palette: TuiPalette) -> Line<'_> {
    Line::from(vec![
        Span::styled("  ", palette.muted()),
        Span::styled(effect.id.as_str(), palette.muted()),
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
    palette: TuiPalette,
) -> Line<'a> {
    let marker_style = if is_selected {
        palette.selected_marker()
    } else {
        palette.muted()
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

fn command_line(command: &str, palette: TuiPalette) -> Line<'_> {
    if command.is_empty() {
        return Line::from("");
    }
    Line::from(vec![
        Span::styled(":", palette.muted()),
        Span::raw(command.to_owned()),
    ])
}

fn input_line<'a>(
    label: String,
    input: &'a TextBuffer,
    command: &str,
    palette: TuiPalette,
) -> Line<'a> {
    if !command.is_empty() {
        return Line::from(vec![
            Span::styled(":", palette.muted()),
            Span::raw(command.to_owned()),
        ]);
    }
    Line::from(vec![
        Span::styled(format!("{label:<8}"), palette.muted()),
        Span::raw(input.as_str()),
    ])
}

fn metadata_line<'a>(label: String, value: &'a str, palette: TuiPalette) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{label:<18}"), palette.muted()),
        Span::raw(value),
    ])
}
