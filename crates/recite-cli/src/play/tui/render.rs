use ratatui::{
    layout::{Constraint, Direction, Layout},
    style::{Color, Modifier, Style},
    text::{Line, Span, Text},
    widgets::{Paragraph, Wrap},
};

use crate::i18n::{Messages, MsgId};
use crate::tui::{KeyHints, Keymap, PromptMode, TextBuffer};

use super::state::{TuiPrompt, TuiState, TuiTranscriptEntry, TuiTranscriptKind, prompt_mode};

pub(super) fn render_tui(frame: &mut ratatui::Frame<'_>, state: &TuiState, messages: &Messages) {
    if prompt_mode(&state.prompt) == PromptMode::Help {
        render_help_overlay(frame, state, messages);
        return;
    }

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

    let transcript = render_transcript(
        &state.transcript,
        chunks[1].width,
        chunks[1].height,
        messages,
    );
    frame.render_widget(
        Paragraph::new(transcript).wrap(Wrap { trim: false }),
        chunks[1],
    );

    frame.render_widget(
        Paragraph::new(render_prompt(&state.prompt, state.keymap, messages))
            .wrap(Wrap { trim: false }),
        chunks[2],
    );

    frame.render_widget(Paragraph::new(render_footer(state, messages)), chunks[3]);
}

fn prompt_height(prompt: &TuiPrompt) -> u16 {
    match prompt {
        TuiPrompt::None | TuiPrompt::Finished { .. } => 2,
        TuiPrompt::Choice { choices, .. } => {
            let visible = choices.iter().filter(|choice| choice.is_visible).count();
            (visible as u16 + 5).clamp(5, 12)
        }
        TuiPrompt::Condition { .. } => 5,
        TuiPrompt::Effect { .. } => 7,
    }
}

fn render_help_overlay(frame: &mut ratatui::Frame<'_>, state: &TuiState, messages: &Messages) {
    let chunks = Layout::default()
        .direction(Direction::Vertical)
        .constraints([Constraint::Min(1), Constraint::Length(1)])
        .split(frame.area());

    frame.render_widget(
        Paragraph::new(help_overlay_lines(&state.prompt, state.keymap, messages)),
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

fn render_transcript<'a>(
    entries: &'a [TuiTranscriptEntry],
    width: u16,
    height: u16,
    messages: &'a Messages,
) -> Text<'a> {
    let mut lines = Vec::new();
    for entry in entries {
        lines.extend(render_transcript_entry(entry, width as usize, messages));
        lines.push(Line::from(""));
    }
    if !lines.is_empty() {
        lines.pop();
    }
    let visible_start = lines.len().saturating_sub(height as usize);
    Text::from(lines.split_off(visible_start))
}

fn render_transcript_entry<'a>(
    entry: &'a TuiTranscriptEntry,
    width: usize,
    messages: &'a Messages,
) -> Vec<Line<'a>> {
    let (label, color) = match entry.kind {
        TuiTranscriptKind::Line => (messages.text(MsgId::TuiTranscriptLine), Color::Green),
        TuiTranscriptKind::Prompt => (messages.text(MsgId::TuiTranscriptPrompt), Color::Blue),
        TuiTranscriptKind::Choice => (messages.text(MsgId::TuiTranscriptChoice), Color::Cyan),
        TuiTranscriptKind::Condition => {
            (messages.text(MsgId::TuiTranscriptCondition), Color::Yellow)
        }
        TuiTranscriptKind::Effect => (messages.text(MsgId::TuiTranscriptEffect), Color::Magenta),
        TuiTranscriptKind::Ack => (messages.text(MsgId::TuiTranscriptAck), Color::Magenta),
        TuiTranscriptKind::Deferred => {
            (messages.text(MsgId::TuiTranscriptDeferred), Color::Magenta)
        }
        TuiTranscriptKind::End => (messages.text(MsgId::TuiTranscriptEnd), Color::DarkGray),
    };
    let mut spans = vec![Span::styled(
        label,
        Style::default().fg(color).add_modifier(Modifier::BOLD),
    )];
    match entry.kind {
        TuiTranscriptKind::Condition | TuiTranscriptKind::Choice | TuiTranscriptKind::Ack => {
            if let Some(id) = entry.id.as_deref() {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(id, Style::default().fg(Color::DarkGray)));
            }
            spans.push(Span::raw(" -> "));
            spans.push(Span::raw(entry.text.as_str()));
            vec![Line::from(spans)]
        }
        TuiTranscriptKind::Effect | TuiTranscriptKind::Deferred => {
            if !entry.text.is_empty() {
                spans.push(Span::raw(" "));
                spans.push(Span::raw(entry.text.as_str()));
            }
            let mut lines = vec![Line::from(spans)];
            if let Some(id) = entry.id.as_deref() {
                lines.push(continuation_metadata("id", id));
            }
            lines
        }
        TuiTranscriptKind::End => {
            if !entry.text.is_empty() {
                spans.push(Span::raw(" "));
                spans.push(Span::raw(entry.text.as_str()));
            }
            vec![Line::from(spans)]
        }
        TuiTranscriptKind::Line | TuiTranscriptKind::Prompt => {
            if let Some(id) = entry.id.as_deref() {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(id, Style::default().fg(Color::DarkGray)));
            }
            let mut lines = vec![Line::from(spans)];
            lines.extend(wrap_continuation(entry.text.as_str(), width, 2));
            lines
        }
    }
}

fn continuation_metadata<'a>(label: &'static str, value: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled("  | ", Style::default().fg(Color::DarkGray)),
        Span::styled(label, Style::default().fg(Color::DarkGray)),
        Span::styled(" ", Style::default().fg(Color::DarkGray)),
        Span::styled(value, Style::default().fg(Color::DarkGray)),
    ])
}

fn wrap_continuation(text: &str, width: usize, indent: usize) -> Vec<Line<'_>> {
    let prefix = if indent == 2 {
        "  | ".to_owned()
    } else {
        " ".repeat(indent)
    };
    let available = width.saturating_sub(prefix.chars().count()).max(8);
    let mut lines = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let separator = usize::from(!current.is_empty());
        if current.chars().count() + separator + word.chars().count() > available
            && !current.is_empty()
        {
            lines.push(Line::from(format!("{prefix}{current}")));
            current.clear();
        }
        if !current.is_empty() {
            current.push(' ');
        }
        current.push_str(word);
    }
    if current.is_empty() && text.is_empty() {
        return lines;
    }
    lines.push(Line::from(format!("{prefix}{current}")));
    lines
}

fn render_prompt<'a>(
    prompt: &'a TuiPrompt,
    keymap: Keymap,
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
                Line::from(Span::styled(
                    messages.text(MsgId::TuiConditionTitle),
                    Style::default()
                        .fg(Color::Yellow)
                        .add_modifier(Modifier::BOLD),
                )),
                Line::from(vec![Span::styled(
                    query.as_str(),
                    Style::default().fg(Color::DarkGray),
                )]),
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
            input,
            command,
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
            lines
        }
    }
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

fn help_overlay_lines(
    prompt: &TuiPrompt,
    keymap: Keymap,
    messages: &Messages,
) -> Vec<Line<'static>> {
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
        help_table_row(
            "? / Esc",
            messages.text(MsgId::TuiHelpActionClose),
            messages.text(MsgId::TuiHelpDescriptionClose),
            false,
        ),
        help_table_row(
            "Ctrl-C",
            messages.text(MsgId::TuiHelpActionQuit),
            messages.text(MsgId::TuiHelpDescriptionInterrupt),
            false,
        ),
    ];
    for control in controls_for_prompt(prompt, keymap) {
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

fn metadata_line<'a>(label: String, value: &'a str) -> Line<'a> {
    Line::from(vec![
        Span::styled(format!("{label:<18}"), Style::default().fg(Color::DarkGray)),
        Span::raw(value),
    ])
}

fn render_footer<'a>(state: &'a TuiState, messages: &'a Messages) -> Line<'a> {
    let help = match state.key_hints {
        KeyHints::Hidden => String::new(),
        KeyHints::Compact => compact_footer_controls(&state.prompt, state.keymap, messages),
        KeyHints::Contextual => contextual_footer_controls(&state.prompt, state.keymap, messages),
    };
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

fn compact_footer_controls(prompt: &TuiPrompt, keymap: Keymap, messages: &Messages) -> String {
    footer_controls(prompt, keymap, messages, true)
}

fn contextual_footer_controls(prompt: &TuiPrompt, keymap: Keymap, messages: &Messages) -> String {
    if prompt_mode(prompt) == PromptMode::Command {
        return messages.text(MsgId::TuiFooterCommand);
    }
    footer_controls(prompt, keymap, messages, false)
}

fn help_footer_control(prompt: &TuiPrompt, keymap: Keymap, messages: &Messages) -> String {
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

#[cfg(test)]
#[path = "render_tests.rs"]
mod tests;
