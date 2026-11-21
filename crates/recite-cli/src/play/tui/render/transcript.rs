use ratatui::text::{Line, Span, Text};
use unicode_width::{UnicodeWidthChar, UnicodeWidthStr};

use crate::i18n::{Messages, MsgId};
use crate::tui::TuiPalette;

use super::super::state::{TuiTranscriptEntry, TuiTranscriptKind};

pub(super) fn render_transcript<'a>(
    entries: impl DoubleEndedIterator<Item = &'a TuiTranscriptEntry>,
    width: u16,
    height: u16,
    messages: &'a Messages,
    palette: TuiPalette,
) -> Text<'a> {
    if height == 0 {
        return Text::from(Vec::<Line<'a>>::new());
    }
    let mut lines = Vec::new();
    let mut previous_spoken = false;
    for entry in entries.rev() {
        // Spoken lines are separated like paragraphs; events between them stay together.
        let spoken = is_spoken(entry.kind);
        if !lines.is_empty() && (spoken || previous_spoken) {
            lines.push(Line::from(""));
        }
        previous_spoken = spoken;
        lines.extend(
            render_transcript_entry(entry, width as usize, messages, palette)
                .into_iter()
                .rev(),
        );
        if lines.len() >= usize::from(height) {
            break;
        }
    }
    lines.truncate(usize::from(height));
    lines.reverse();
    Text::from(lines)
}

fn is_spoken(kind: TuiTranscriptKind) -> bool {
    matches!(kind, TuiTranscriptKind::Line | TuiTranscriptKind::Prompt)
}

fn render_transcript_entry<'a>(
    entry: &'a TuiTranscriptEntry,
    width: usize,
    messages: &'a Messages,
    palette: TuiPalette,
) -> Vec<Line<'a>> {
    let mut spans = vec![match entry.speaker.as_deref() {
        Some(speaker) if is_spoken(entry.kind) => speaker_span(speaker, palette),
        _ => transcript_label_span(entry.kind, messages, palette),
    }];
    match entry.kind {
        // The runtime's effect IDs belong to the acknowledgement panel and `recite trace`;
        // the transcript says what was asked and what happened.
        TuiTranscriptKind::Effect | TuiTranscriptKind::Deferred | TuiTranscriptKind::Ack => {
            if !entry.text.is_empty() {
                spans.push(Span::raw(" "));
                spans.push(Span::raw(entry.text.as_str()));
            }
            vec![Line::from(spans)]
        }
        TuiTranscriptKind::Choice if !entry.text.is_empty() => {
            spans.push(Span::raw(" "));
            spans.push(Span::raw(entry.text.as_str()));
            if let Some(id) = entry.id.as_deref() {
                spans.push(Span::raw("  "));
                spans.push(Span::styled(id.to_owned(), palette.muted()));
            }
            vec![Line::from(spans)]
        }
        TuiTranscriptKind::Condition | TuiTranscriptKind::Choice => {
            if let Some(id) = entry.id.as_deref() {
                spans.push(Span::raw(" "));
                spans.push(Span::styled(id.to_owned(), palette.muted()));
            }
            if !entry.text.is_empty() {
                spans.push(Span::raw(" -> "));
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
                spans.push(Span::styled(id.to_owned(), palette.muted()));
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
    let available = width.saturating_sub(prefix.width()).max(8);
    if text.is_empty() {
        return Vec::new();
    }
    wrap_display(text, available)
        .into_iter()
        .map(|row| Line::from(format!("{prefix}{row}")))
        .collect()
}

/// Wrap by terminal display width: CJK and other wide characters take two columns, and a run
/// with no spaces (as in Chinese or Japanese) breaks between characters.
pub(super) fn wrap_display(text: &str, available: usize) -> Vec<String> {
    let mut rows = Vec::new();
    let mut current = String::new();
    for word in text.split_whitespace() {
        let separator = usize::from(!current.is_empty());
        if current.width() + separator + word.width() <= available {
            if separator == 1 {
                current.push(' ');
            }
            current.push_str(word);
            continue;
        }
        if !current.is_empty() && word.width() <= available {
            rows.push(std::mem::take(&mut current));
            current.push_str(word);
            continue;
        }
        if separator == 1 && current.width() + 1 < available {
            current.push(' ');
        }
        for ch in word.chars() {
            if current.width() + ch.width().unwrap_or(0) > available {
                rows.push(std::mem::take(&mut current));
            }
            current.push(ch);
        }
    }
    if !current.is_empty() {
        rows.push(current);
    }
    rows
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

/// A speaker's name in capitals, as Writer's margin labels set it.
fn speaker_span(speaker: &str, palette: TuiPalette) -> Span<'static> {
    Span::styled(speaker.to_uppercase(), palette.title())
}

pub(super) fn prompt_header_line<'a>(
    kind: TuiTranscriptKind,
    id: Option<&'a str>,
    speaker: Option<&str>,
    messages: &Messages,
    palette: TuiPalette,
) -> Line<'a> {
    let mut spans = vec![match speaker {
        Some(speaker) => speaker_span(speaker, palette),
        None => transcript_label_span(kind, messages, palette),
    }];
    if let Some(id) = id {
        spans.push(Span::raw(" "));
        spans.push(Span::styled(id.to_owned(), palette.muted()));
    }
    Line::from(spans)
}

#[cfg(test)]
mod tests;
