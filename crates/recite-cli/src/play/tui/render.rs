use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span},
    widgets::{Paragraph, Wrap},
};

use crate::i18n::{Messages, MsgId};
use crate::tui::{KeyHints, PromptMode, TextBuffer};

use super::state::{TuiPrompt, TuiState, TuiTranscriptEntry, TuiTranscriptKind, prompt_mode};

pub(super) fn render_tui(frame: &mut ratatui::Frame<'_>, state: &TuiState, messages: &Messages) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([
            Constraint::Length(2),
            Constraint::Min(5),
            Constraint::Length(prompt_height(&state.prompt)),
            Constraint::Length(1),
        ])
        .split(frame.area());

    let header = Line::from(vec![
        Span::styled(
            messages.text(MsgId::TuiHeaderTitle),
            Style::default()
                .fg(Color::Cyan)
                .add_modifier(Modifier::BOLD),
        ),
        Span::raw("  "),
        Span::styled(
            format!("{} ", messages.text(MsgId::TuiHeaderAsset)),
            Style::default().fg(Color::DarkGray),
        ),
        Span::raw(state.asset.as_str()),
        Span::raw("  "),
        Span::styled(
            format!("{} ", messages.text(MsgId::TuiHeaderBlock)),
            Style::default().fg(Color::DarkGray),
        ),
        Span::styled(
            state.block.as_str(),
            Style::default().add_modifier(Modifier::BOLD),
        ),
    ]);
    frame.render_widget(Paragraph::new(vec![header, Line::from("")]), chunks[0]);

    let visible_transcript = state
        .transcript
        .iter()
        .rev()
        .take(chunks[1].height as usize)
        .rev()
        .collect::<Vec<_>>();
    let id_width = transcript_id_width(&visible_transcript);
    let transcript = visible_transcript
        .iter()
        .map(|entry| render_transcript_entry(entry, id_width, messages))
        .collect::<Vec<_>>();
    frame.render_widget(
        Paragraph::new(transcript).wrap(Wrap { trim: false }),
        chunks[1],
    );

    frame.render_widget(
        Paragraph::new(render_prompt(&state.prompt, messages)).wrap(Wrap { trim: false }),
        chunks[2],
    );

    frame.render_widget(Paragraph::new(render_footer(state, messages)), chunks[3]);
}

fn prompt_height(prompt: &TuiPrompt) -> u16 {
    match prompt {
        TuiPrompt::None | TuiPrompt::Finished => 2,
        TuiPrompt::Choice { choices, .. } => {
            let visible = choices.iter().filter(|choice| choice.is_visible).count();
            (visible as u16 + 5).clamp(5, 12)
        }
        TuiPrompt::Condition { show_help, .. } => {
            if *show_help {
                6
            } else {
                4
            }
        }
        TuiPrompt::Effect { show_help, .. } => {
            if *show_help {
                9
            } else {
                7
            }
        }
    }
}

fn transcript_id_width(entries: &[&TuiTranscriptEntry]) -> usize {
    entries
        .iter()
        .filter_map(|entry| entry.id.as_ref())
        .map(|id| id.chars().count().min(32))
        .max()
        .unwrap_or(12)
        .clamp(12, 32)
}

fn render_transcript_entry<'a>(
    entry: &'a TuiTranscriptEntry,
    id_width: usize,
    messages: &'a Messages,
) -> Line<'a> {
    let (label, color) = match entry.kind {
        TuiTranscriptKind::Line => (messages.text(MsgId::TuiTranscriptLine), Color::Green),
        TuiTranscriptKind::Prompt => (messages.text(MsgId::TuiTranscriptPrompt), Color::Blue),
        TuiTranscriptKind::Choice => (messages.text(MsgId::TuiTranscriptChoice), Color::Cyan),
        TuiTranscriptKind::Condition => {
            (messages.text(MsgId::TuiTranscriptCondition), Color::Yellow)
        }
        TuiTranscriptKind::Effect => (messages.text(MsgId::TuiTranscriptEffect), Color::Magenta),
        TuiTranscriptKind::Ack => (messages.text(MsgId::TuiTranscriptAck), Color::Magenta),
        TuiTranscriptKind::End => (messages.text(MsgId::TuiTranscriptEnd), Color::DarkGray),
    };
    let mut spans = vec![Span::styled(
        format!("{label:<9}"),
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )];
    let id = entry
        .id
        .as_deref()
        .map(|id| clamp_display(id, id_width))
        .unwrap_or_else(|| String::from(""));
    spans.push(Span::styled(
        format!("{id:<id_width$}"),
        Style::default().fg(Color::DarkGray),
    ));
    spans.push(Span::raw("  "));
    spans.push(Span::raw(entry.text.as_str()));
    Line::from(spans)
}

fn clamp_display(value: &str, max_width: usize) -> String {
    let char_count = value.chars().count();
    if char_count <= max_width {
        return value.to_owned();
    }
    if max_width <= 3 {
        return ".".repeat(max_width);
    }
    let prefix = value.chars().take(max_width - 3).collect::<String>();
    format!("{prefix}...")
}

fn render_prompt<'a>(prompt: &'a TuiPrompt, messages: &'a Messages) -> Vec<Line<'a>> {
    match prompt {
        TuiPrompt::None => vec![Line::from(Span::styled(
            messages.text(MsgId::TuiWaiting),
            Style::default().fg(Color::DarkGray),
        ))],
        TuiPrompt::Finished => vec![Line::from("")],
        TuiPrompt::Condition {
            query,
            input,
            command,
            show_help,
            ..
        } => {
            let mut lines = vec![
                Line::from(Span::styled(
                    messages.text(MsgId::TuiConditionTitle),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(vec![
                    Span::styled(query.as_str(), Style::default().fg(Color::DarkGray)),
                    Span::raw("  "),
                    Span::raw("y/n"),
                ]),
                input_line(messages.text(MsgId::TuiInputAnswer), input, command),
            ];
            if *show_help {
                lines.extend(help_lines("condition", messages));
            }
            lines
        }
        TuiPrompt::Effect {
            mode,
            id,
            function,
            args,
            input,
            command,
            show_help,
            ..
        } => {
            let mut lines = vec![
                Line::from(Span::styled(
                    messages.text(MsgId::TuiEffectTitle),
                    Style::default()
                        .fg(Color::Magenta)
                        .add_modifier(Modifier::BOLD),
                )),
                metadata_line(messages.text(MsgId::TuiMetadataMode), mode),
                metadata_line(messages.text(MsgId::TuiMetadataRuntimeEffectId), id),
                metadata_line(messages.text(MsgId::TuiMetadataFunction), function),
                metadata_line(messages.text(MsgId::TuiMetadataArgs), args),
                input_line(messages.text(MsgId::TuiInputAck), input, command),
            ];
            if *show_help {
                lines.extend(help_lines("effect", messages));
            }
            lines
        }
        TuiPrompt::Choice {
            line,
            choices,
            selected,
            input,
            command,
            show_help,
            ..
        } => {
            let mut lines = Vec::new();
            if let Some(line) = line {
                lines.push(Line::from(vec![
                    Span::styled(line.id.as_str(), Style::default().fg(Color::DarkGray)),
                    Span::raw("  "),
                    Span::styled(
                        line.text.as_str(),
                        Style::default().add_modifier(Modifier::BOLD),
                    ),
                ]));
            } else {
                lines.push(Line::from(Span::styled(
                    messages.text(MsgId::TuiChoiceTitle),
                    Style::default().add_modifier(Modifier::BOLD),
                )));
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
                let selected_style = if is_selected {
                    Style::default()
                        .fg(Color::Cyan)
                        .add_modifier(Modifier::BOLD)
                } else {
                    Style::default().fg(Color::DarkGray)
                };
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
                    Span::styled(marker, selected_style),
                    Span::raw(" "),
                    Span::styled(
                        format!("{:>2}", choice.index),
                        Style::default()
                            .fg(Color::Cyan)
                            .add_modifier(Modifier::BOLD),
                    ),
                    Span::raw("  "),
                    Span::styled(
                        format!("{:<16}", choice.id),
                        Style::default().fg(Color::DarkGray),
                    ),
                    Span::styled(choice.text.as_str(), style),
                    Span::styled(suffix, Style::default().fg(Color::DarkGray)),
                ]));
            }
            lines.push(input_line(
                messages.text(MsgId::TuiInputChoice),
                input,
                command,
            ));
            if *show_help {
                lines.extend(help_lines("choice", messages));
            }
            lines
        }
    }
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

fn help_lines(context: &str, messages: &Messages) -> Vec<Line<'static>> {
    vec![
        Line::from(""),
        Line::from(vec![
            Span::styled(
                messages.text(MsgId::TuiHelpLabel),
                Style::default().fg(Color::DarkGray),
            ),
            Span::raw(" "),
            Span::raw(match context {
                "choice" => messages.text(MsgId::TuiHelpChoice),
                "condition" => messages.text(MsgId::TuiHelpCondition),
                "effect" => messages.text(MsgId::TuiHelpEffect),
                _ => messages.text(MsgId::TuiHelpDefault),
            }),
        ]),
    ]
}

fn metadata_line<'a>(label: String, value: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{label:<18}"), Style::default().fg(Color::DarkGray)),
        Span::raw(value),
    ])
}

fn render_footer<'a>(state: &'a TuiState, messages: &'a Messages) -> Line<'a> {
    let help = match state.key_hints {
        KeyHints::Hidden => String::new(),
        KeyHints::Compact => match state.prompt {
            TuiPrompt::Choice { .. } => messages.text(MsgId::TuiFooterCompactChoice),
            TuiPrompt::Condition { .. } => messages.text(MsgId::TuiFooterCompactCondition),
            TuiPrompt::Effect { .. } => messages.text(MsgId::TuiFooterCompactEffect),
            TuiPrompt::Finished => messages.text(MsgId::TuiFooterCompactFinished),
            TuiPrompt::None => String::new(),
        },
        KeyHints::Contextual => match &state.prompt {
            _ if prompt_mode(&state.prompt) == PromptMode::Help => {
                messages.text(MsgId::TuiFooterHelp)
            }
            TuiPrompt::Choice { mode, .. } => match mode {
                PromptMode::Normal => messages.text(MsgId::TuiFooterChoiceNormal),
                PromptMode::Insert => messages.text(MsgId::TuiFooterChoiceInsert),
                PromptMode::Command => messages.text(MsgId::TuiFooterCommand),
                PromptMode::Help => messages.text(MsgId::TuiFooterHelp),
            },
            TuiPrompt::Condition { mode, .. } => match mode {
                PromptMode::Command => messages.text(MsgId::TuiFooterCommand),
                PromptMode::Help => messages.text(MsgId::TuiFooterHelp),
                _ => messages.text(MsgId::TuiFooterCondition),
            },
            TuiPrompt::Effect { input_mode, .. } => match input_mode {
                PromptMode::Command => messages.text(MsgId::TuiFooterCommand),
                PromptMode::Help => messages.text(MsgId::TuiFooterHelp),
                _ => messages.text(MsgId::TuiFooterEffect),
            },
            TuiPrompt::Finished => messages.text(MsgId::TuiFooterFinished),
            TuiPrompt::None => String::new(),
        },
    };
    if help.is_empty() {
        return Line::from(Span::styled(
            state.status.as_str(),
            Style::default().fg(Color::DarkGray),
        ));
    }
    Line::from(vec![
        Span::styled(state.status.as_str(), Style::default().fg(Color::DarkGray)),
        Span::raw("  "),
        Span::raw(help),
    ])
}

#[cfg(test)]
mod tests {
    use super::*;
    use ratatui::Terminal;

    use super::super::state::{TuiChoiceRow, TuiPromptLine};

    #[test]
    fn transcript_ids_are_aligned_and_clamped() {
        let entries = [
            TuiTranscriptEntry {
                kind: TuiTranscriptKind::Line,
                id: Some("short".to_owned()),
                text: "Line.".to_owned(),
            },
            TuiTranscriptEntry {
                kind: TuiTranscriptKind::Effect,
                id: Some("effect:very-long-source-location:123:45#9".to_owned()),
                text: "blocking grant_item (map)".to_owned(),
            },
        ];
        let visible = entries.iter().collect::<Vec<_>>();
        let width = transcript_id_width(&visible);

        assert_eq!(width, 32);
        let messages = Messages::load(&crate::i18n::UiLocale::default()).expect("messages");
        assert!(
            format!(
                "{:?}",
                render_transcript_entry(&entries[1], width, &messages)
            )
            .contains("...")
        );
    }

    #[test]
    fn tui_render_includes_header_and_choice_prompt() {
        let state = TuiState {
            asset: "asset".to_owned(),
            block: "start".to_owned(),
            transcript: vec![TuiTranscriptEntry {
                kind: TuiTranscriptKind::Line,
                id: Some("intro".to_owned()),
                text: "Welcome.".to_owned(),
            }],
            prompt: TuiPrompt::Choice {
                line: Some(TuiPromptLine {
                    id: "intro".to_owned(),
                    text: "Welcome.".to_owned(),
                }),
                choices: vec![TuiChoiceRow {
                    index: 1,
                    id: "help".to_owned(),
                    text: "Help.".to_owned(),
                    is_available: true,
                    unavailable_reason: None,
                    is_visible: true,
                }],
                selected: 0,
                mode: PromptMode::Insert,
                input: TextBuffer::default(),
                command: TextBuffer::default(),
                show_help: false,
            },
            status: "choice> ".to_owned(),
            key_hints: KeyHints::Contextual,
        };
        let content = render_tui_content(&state, 80, 20);

        assert!(content.contains("recite play"));
        assert!(content.contains("asset"));
        assert!(content.contains("block"));
        assert!(content.contains("intro"));
        assert!(content.contains("Welcome."));
        assert!(content.contains("help"));
        assert!(content.contains("Help."));
        assert!(content.contains("Type choice ID/index"));
    }

    #[test]
    fn tui_render_finished_state_without_inactive_prompt_filler() {
        let state = TuiState {
            asset: "asset".to_owned(),
            block: "start".to_owned(),
            transcript: vec![
                TuiTranscriptEntry {
                    kind: TuiTranscriptKind::Choice,
                    id: Some("help".to_owned()),
                    text: "selected".to_owned(),
                },
                TuiTranscriptEntry {
                    kind: TuiTranscriptKind::Line,
                    id: Some("helped".to_owned()),
                    text: "Helped.".to_owned(),
                },
                TuiTranscriptEntry {
                    kind: TuiTranscriptKind::End,
                    id: None,
                    text: "end".to_owned(),
                },
            ],
            prompt: TuiPrompt::Finished,
            status: "finished".to_owned(),
            key_hints: KeyHints::Contextual,
        };
        let content = render_tui_content(&state, 80, 20);

        assert!(content.contains("choice"));
        assert!(content.contains("help"));
        assert!(content.contains("line"));
        assert!(content.contains("Helped."));
        assert!(content.contains("end"));
        assert!(content.contains("Enter/Esc/q to exit"));
        assert!(!content.contains("No active prompt"));
    }

    #[test]
    fn tui_render_footer_uses_effective_help_mode() {
        let state = TuiState {
            asset: "asset".to_owned(),
            block: "start".to_owned(),
            transcript: Vec::new(),
            prompt: TuiPrompt::Condition {
                query: "trusts(player)".to_owned(),
                mode: PromptMode::Insert,
                input: TextBuffer::default(),
                command: TextBuffer::default(),
                show_help: true,
            },
            status: "answer> ".to_owned(),
            key_hints: KeyHints::Contextual,
        };
        let content = render_tui_content(&state, 80, 20);

        assert!(content.contains("Esc closes help"));
    }

    #[test]
    fn tui_render_stays_structured_on_narrow_terminal() {
        let state = TuiState {
            asset: "/tmp/recite-play.recitec".to_owned(),
            block: "start".to_owned(),
            transcript: vec![TuiTranscriptEntry {
                kind: TuiTranscriptKind::Effect,
                id: Some("grant#1".to_owned()),
                text: "blocking grant_item (map)".to_owned(),
            }],
            prompt: TuiPrompt::Effect {
                mode: "blocking".to_owned(),
                id: "grant#1".to_owned(),
                function: "grant_item".to_owned(),
                args: "(map)".to_owned(),
                input_mode: PromptMode::Insert,
                input: TextBuffer::default(),
                command: TextBuffer::default(),
                show_help: false,
            },
            status: "ack grant#1 with Enter or ack".to_owned(),
            key_hints: KeyHints::Contextual,
        };
        let content = render_tui_content(&state, 60, 16);

        assert!(content.contains("recite play"));
        assert!(content.contains("Blocking Effect"));
        assert!(content.contains("runtime effect ID"));
        assert!(content.contains("grant#1"));
        assert!(content.contains("Enter or ack"));
    }

    fn render_tui_content(state: &TuiState, width: u16, height: u16) -> String {
        let backend = ratatui::backend::TestBackend::new(width, height);
        let mut terminal = Terminal::new(backend).expect("terminal");
        let messages = Messages::load(&crate::i18n::UiLocale::default()).expect("messages");

        terminal
            .draw(|frame| render_tui(frame, state, &messages))
            .expect("draw");
        terminal
            .backend()
            .buffer()
            .content
            .iter()
            .map(|cell| cell.symbol())
            .collect::<String>()
    }
}
