use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::Modifier,
    text::{Line, Span, Text},
    widgets::{Paragraph, Wrap},
};

use crate::i18n::{Messages, MsgId};
use crate::tui::{PromptMode, TuiPalette};

use super::state::{TuiPrompt, TuiState, prompt_mode};

mod controls;
mod footer;
mod help;
mod prompt;
mod transcript;

use footer::{help_footer_control, render_footer};
use help::help_overlay_lines;
use prompt::{render_deferred_queue, render_prompt};

pub(super) fn render_tui(frame: &mut ratatui::Frame<'_>, state: &TuiState, messages: &Messages) {
    if prompt_mode(&state.prompt) == PromptMode::Help {
        render_help_overlay(frame, state, messages);
        return;
    }

    let area = frame.area();
    let queue_height = deferred_queue_height(state);
    let repeat_line = !prompt_line_is_last_transcript_entry(state);
    let prompt_height = prompt_height(&state.prompt, area.width, repeat_line)
        .min(available_prompt_height(area.height, queue_height));
    // Transcript and prompt read top-down as one flow; spare rows collect above the footer.
    let transcript_room = area
        .height
        .saturating_sub(2 + queue_height + prompt_height + 1 + 1);
    let transcript = transcript::render_transcript(
        state.transcript.iter(),
        area.width,
        transcript_room,
        messages,
        state.palette,
    );
    let transcript_rows = transcript_height(&transcript, area.width, transcript_room);
    let gap = u16::from(transcript_rows > 0);
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Length(transcript_rows),
            Constraint::Length(gap),
            Constraint::Length(queue_height),
            Constraint::Length(prompt_height),
            Constraint::Min(0),
            Constraint::Length(1),
        ])
        .split(area);
    let chunks = [chunks[0], chunks[1], chunks[3], chunks[4], chunks[6]];

    frame.render_widget(
        Paragraph::new(header_lines(state, messages, state.palette)),
        chunks[0],
    );

    frame.render_widget(
        Paragraph::new(transcript).wrap(Wrap { trim: false }),
        chunks[1],
    );

    if queue_height > 0 {
        frame.render_widget(
            Paragraph::new(render_deferred_queue(state, messages)).wrap(Wrap { trim: false }),
            chunks[2],
        );
    }

    frame.render_widget(
        Paragraph::new(render_prompt(
            &state.prompt,
            repeat_line,
            state.keymap,
            chunks[3].width,
            messages,
            state.palette,
        ))
        .wrap(Wrap { trim: false }),
        chunks[3],
    );

    frame.render_widget(
        Paragraph::new(render_footer(state, messages, chunks[4].width)),
        chunks[4],
    );
}

fn header_lines<'a>(
    state: &'a TuiState,
    messages: &Messages,
    palette: TuiPalette,
) -> Vec<Line<'a>> {
    vec![
        Line::from(vec![
            Span::styled(messages.text(MsgId::TuiHeaderTitle), palette.title()),
            Span::raw("  "),
            Span::styled(
                format!("{} ", messages.text(MsgId::TuiHeaderAsset)),
                palette.muted(),
            ),
            Span::raw(state.asset.as_str()),
            Span::raw("  "),
            Span::styled(
                format!("{} ", messages.text(MsgId::TuiHeaderBlock)),
                palette.muted(),
            ),
            Span::styled(
                state.block.as_str(),
                palette.plain().add_modifier(Modifier::BOLD),
            ),
        ]),
        Line::from(""),
    ]
}

/// Rows the transcript needs at this width, capped by the room left above the prompt.
fn transcript_height(lines: &Text<'_>, width: u16, room: u16) -> u16 {
    if room == 0 || lines.lines.is_empty() {
        return 0;
    }
    let width = usize::from(width.max(1));
    let rows: usize = lines
        .lines
        .iter()
        .map(|line| line.width().max(1).div_ceil(width))
        .sum();
    u16::try_from(rows).unwrap_or(u16::MAX).min(room)
}

/// The prompt's line was just shown as the last transcript entry, so its text is not repeated.
fn prompt_line_is_last_transcript_entry(state: &TuiState) -> bool {
    let TuiPrompt::Choice {
        line: Some(line), ..
    } = &state.prompt
    else {
        return false;
    };
    state
        .transcript
        .last()
        .and_then(|entry| entry.id.as_deref())
        .is_some_and(|id| id == line.id)
}

fn prompt_height(prompt: &TuiPrompt, width: u16, repeat_line: bool) -> u16 {
    match prompt {
        TuiPrompt::None | TuiPrompt::Finished { .. } => 2,
        TuiPrompt::Choice { line, choices, .. } => {
            let prompt_lines = line
                .as_ref()
                .filter(|_| repeat_line)
                .map(|line| 1 + wrapped_line_count(line.text.as_str(), width as usize, 2))
                .unwrap_or(1);
            let visible = choices.iter().filter(|choice| choice.is_visible).count();
            (prompt_lines + visible + 1).clamp(5, u16::MAX as usize) as u16
        }
        TuiPrompt::Condition { .. } => 5,
        TuiPrompt::EnumCondition { .. } => 4,
        TuiPrompt::Effect { .. } => 7,
    }
}

fn available_prompt_height(frame_height: u16, queue_height: u16) -> u16 {
    frame_height
        .saturating_sub(2)
        .saturating_sub(queue_height)
        .saturating_sub(5)
        .saturating_sub(1)
        .max(2)
}

fn wrapped_line_count(text: &str, width: usize, indent: usize) -> usize {
    if text.is_empty() {
        return 0;
    }
    let prefix_width = if indent == 2 { 4 } else { indent };
    transcript::wrap_display(text, width.saturating_sub(prefix_width).max(8)).len()
}

fn deferred_queue_height(state: &TuiState) -> u16 {
    if state.deferred_queue_expanded && !state.deferred_queue.is_empty() {
        (state.deferred_queue.len() as u16 + 1).clamp(2, 6)
    } else {
        0
    }
}

fn render_help_overlay(frame: &mut ratatui::Frame<'_>, state: &TuiState, messages: &Messages) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(frame.area());

    frame.render_widget(
        Paragraph::new(help_overlay_lines(state, messages)),
        chunks[0],
    );
    frame.render_widget(
        Paragraph::new(Line::from(help_footer_control(
            &state.prompt,
            state.keymap,
            messages,
        ))),
        chunks[1],
    );
}

#[cfg(test)]
mod tests;
