use ratatui::text::{Line, Span, Text};

use crate::i18n::{Messages, MsgId};
use crate::tui::TuiPalette;

use super::super::state::{TuiTranscriptEntry, TuiTranscriptKind};

pub(super) fn render_transcript<'a>(
    entries: &'a [TuiTranscriptEntry],
    width: u16,
    height: u16,
    messages: &'a Messages,
    palette: TuiPalette,
) -> Text<'a> {
    let mut lines = Vec::new();
    for entry in entries {
        lines.extend(render_transcript_entry(
            entry,
            width as usize,
            messages,
            palette,
        ));
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
    palette: TuiPalette,
) -> Vec<Line<'a>> {
    let mut spans = vec![transcript_label_span(entry.kind, messages, palette)];
    match entry.kind {
        TuiTranscriptKind::Condition | TuiTranscriptKind::Choice | TuiTranscriptKind::Ack => {
            if let Some(id) = entry.id.as_deref() {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(id, palette.muted()));
            }
            if !entry.text.is_empty() {
                spans.push(Span::raw(" -> "));
                spans.push(Span::raw(entry.text.as_str()));
            }
            vec![Line::from(spans)]
        }
        TuiTranscriptKind::Effect | TuiTranscriptKind::Deferred => {
            if let Some(id) = entry.id.as_deref() {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(id, palette.muted()));
            }
            if !entry.text.is_empty() {
                spans.push(Span::raw(" "));
                spans.push(Span::raw(entry.text.as_str()));
            }
            vec![Line::from(spans)]
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
                spans.push(Span::styled(id, palette.muted()));
            }
            let mut lines = vec![Line::from(spans)];
            lines.extend(wrap_continuation(entry.text.as_str(), width, 2));
            lines
        }
    }
}

pub(super) fn wrap_continuation(text: &str, width: usize, indent: usize) -> Vec<Line<'_>> {
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

fn transcript_label_span(
    kind: TuiTranscriptKind,
    messages: &Messages,
    palette: TuiPalette,
) -> Span<'static> {
    let (label, style) = transcript_label(kind, messages, palette);
    Span::styled(label, style)
}

pub(super) fn transcript_label(
    kind: TuiTranscriptKind,
    messages: &Messages,
    palette: TuiPalette,
) -> (String, ratatui::style::Style) {
    let label = match kind {
        TuiTranscriptKind::Line => messages.text(MsgId::TuiTranscriptLine),
        TuiTranscriptKind::Prompt => messages.text(MsgId::TuiTranscriptPrompt),
        TuiTranscriptKind::Choice => messages.text(MsgId::TuiTranscriptChoice),
        TuiTranscriptKind::Condition => messages.text(MsgId::TuiTranscriptCondition),
        TuiTranscriptKind::Effect => messages.text(MsgId::TuiTranscriptEffect),
        TuiTranscriptKind::Ack => messages.text(MsgId::TuiTranscriptAck),
        TuiTranscriptKind::Deferred => messages.text(MsgId::TuiTranscriptDeferred),
        TuiTranscriptKind::End => messages.text(MsgId::TuiTranscriptEnd),
    };
    let style = match kind {
        TuiTranscriptKind::Line => palette.line_label(),
        TuiTranscriptKind::Prompt => palette.prompt_label(),
        TuiTranscriptKind::Choice => palette.choice_label(),
        TuiTranscriptKind::Condition => palette.condition_label(),
        TuiTranscriptKind::Effect | TuiTranscriptKind::Ack | TuiTranscriptKind::Deferred => {
            palette.effect_label()
        }
        TuiTranscriptKind::End => palette.muted(),
    };
    (label, style)
}

pub(super) fn prompt_header_line<'a>(
    kind: TuiTranscriptKind,
    id: Option<&'a str>,
    messages: &Messages,
    palette: TuiPalette,
) -> Line<'a> {
    let mut spans = vec![transcript_label_span(kind, messages, palette)];
    if let Some(id) = id {
        spans.push(Span::raw(" "));
        spans.push(Span::styled(id.to_owned(), palette.muted()));
    }
    Line::from(spans)
}
