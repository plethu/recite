//! The human `recite run` transcript: structured entries collected during a fixture run, laid
//! out as a script with a label column, the words, and a right-hand column for IDs and results.
//!
//! Labels are speakers (plum capitals, as in Writer's margin) or event kinds (teal). Choices list
//! every option with the fixture's pick marked. On a terminal the words wrap at word boundaries
//! under their own column; anywhere else each entry stays on one line so the output greps and
//! diffs cleanly. `recite trace` remains the machine-readable record.
use std::io::Write;

use unicode_width::UnicodeWidthStr;

use crate::error::CliError;
use crate::i18n::{Messages, MsgId};
use crate::ink::Ink;

/// Wider terminals keep this measure so the right-hand column stays near the words.
const MAX_COLUMNS: usize = 100;
const LABEL_MIN: usize = 9;
const LABEL_MAX: usize = 14;
const GAP: usize = 2;
/// The narrowest column of words worth wrapping into.
const MIN_MEASURE: usize = 20;

#[derive(Debug, Default)]
pub(crate) struct RunTranscript {
    entries: Vec<RunEntry>,
}

#[derive(Debug)]
enum RunEntry {
    Effect {
        id: String,
        call: String,
        mode: String,
        acknowledged: bool,
    },
    Condition {
        query: String,
        result: String,
    },
    Line(RunLine),
    Prompt {
        line: Option<RunLine>,
        choices: Vec<RunChoice>,
        selected: Option<String>,
    },
    End {
        deferred: Vec<String>,
    },
}

#[derive(Debug)]
pub(crate) struct RunLine {
    pub(crate) id: String,
    pub(crate) speaker: Option<String>,
    pub(crate) text: String,
}

#[derive(Debug)]
pub(crate) struct RunChoice {
    pub(crate) id: String,
    pub(crate) text: String,
    pub(crate) available: bool,
}

impl RunTranscript {
    pub(crate) fn effect(&mut self, id: String, call: String, mode: String) {
        self.entries.push(RunEntry::Effect {
            id,
            call,
            mode,
            acknowledged: false,
        });
    }

    /// Marks the most recent request with this ID; acknowledgements follow their request.
    pub(crate) fn acknowledge(&mut self, effect_id: &str) {
        let pending = self.entries.iter_mut().rev().find_map(|entry| match entry {
            RunEntry::Effect {
                id, acknowledged, ..
            } if id == effect_id => Some(acknowledged),
            _ => None,
        });
        if let Some(acknowledged) = pending {
            *acknowledged = true;
        }
    }

    pub(crate) fn condition(&mut self, query: String, result: String) {
        self.entries.push(RunEntry::Condition { query, result });
    }

    pub(crate) fn line(&mut self, line: RunLine) {
        self.entries.push(RunEntry::Line(line));
    }

    pub(crate) fn prompt(&mut self, line: Option<RunLine>, choices: Vec<RunChoice>) {
        self.entries.push(RunEntry::Prompt {
            line,
            choices,
            selected: None,
        });
    }

    /// Records the fixture's pick against the latest prompt that offered it.
    pub(crate) fn select(&mut self, choice_id: &str) {
        let prompt = self.entries.iter_mut().rev().find_map(|entry| match entry {
            RunEntry::Prompt {
                choices, selected, ..
            } if choices.iter().any(|choice| choice.id == choice_id) => Some(selected),
            _ => None,
        });
        if let Some(selected) = prompt {
            *selected = Some(choice_id.to_owned());
        }
    }

    pub(crate) fn end(&mut self, deferred: Vec<String>) {
        self.entries.push(RunEntry::End { deferred });
    }

    /// `columns` is the terminal width, or `None` to keep each entry on one line.
    pub(crate) fn write(
        &self,
        writer: &mut dyn Write,
        messages: &Messages,
        ink: Ink,
        columns: Option<usize>,
    ) -> Result<(), CliError> {
        let rows: Vec<Row> = self
            .entries
            .iter()
            .flat_map(|entry| self.rows(entry, messages))
            .collect();
        let layout = Layout::new(self.label_width(messages), columns);
        for row in &rows {
            layout.write(writer, ink, row)?;
        }
        Ok(())
    }

    fn label_width(&self, messages: &Messages) -> usize {
        let kinds = [
            MsgId::RunLabelEffect,
            MsgId::RunLabelCondition,
            MsgId::RunLabelChoose,
            MsgId::RunLabelEnd,
            MsgId::RunLabelDeferred,
        ]
        .map(|id| messages.text(id).width());
        let speakers = self.entries.iter().flat_map(|entry| match entry {
            RunEntry::Line(line) => line.speaker.as_deref().map(str::width),
            RunEntry::Prompt { line, .. } => line
                .as_ref()
                .and_then(|line| line.speaker.as_deref())
                .map(str::width),
            _ => None,
        });
        kinds
            .into_iter()
            .chain(speakers)
            .max()
            .unwrap_or(LABEL_MIN)
            .clamp(LABEL_MIN, LABEL_MAX)
    }

    fn rows(&self, entry: &RunEntry, messages: &Messages) -> Vec<Row> {
        match entry {
            RunEntry::Effect {
                call,
                mode,
                acknowledged,
                ..
            } => {
                let trailing = if *acknowledged {
                    messages.format(MsgId::RunEffectAcknowledged, [("mode", mode.as_str())])
                } else {
                    messages.format(MsgId::RunEffectMode, [("mode", mode.as_str())])
                };
                vec![Row::kind(
                    messages.text(MsgId::RunLabelEffect),
                    call,
                    trailing,
                )]
            }
            RunEntry::Condition { query, result } => vec![Row::kind(
                messages.text(MsgId::RunLabelCondition),
                query,
                result.clone(),
            )],
            RunEntry::Line(line) => vec![Row::line(line)],
            RunEntry::Prompt {
                line,
                choices,
                selected,
            } => {
                let mut rows = match line {
                    Some(line) => vec![Row::line(line)],
                    None => vec![Row::kind(
                        messages.text(MsgId::RunLabelChoose),
                        "",
                        String::new(),
                    )],
                };
                for (index, choice) in choices.iter().enumerate() {
                    let is_selected = selected.as_deref() == Some(choice.id.as_str());
                    let mut text = choice.text.clone();
                    if !choice.available {
                        text = format!("{text} ({})", messages.text(MsgId::RunChoiceUnavailable));
                    }
                    rows.push(Row {
                        label: Label::Choice {
                            number: index + 1,
                            selected: is_selected,
                        },
                        text,
                        trailing: choice.id.clone(),
                    });
                }
                rows
            }
            RunEntry::End { deferred } => {
                let mut rows = vec![Row::kind(
                    messages.text(MsgId::RunLabelEnd),
                    &messages.text(MsgId::RunEndText),
                    String::new(),
                )];
                rows.extend(deferred.iter().map(|call| {
                    Row::kind(messages.text(MsgId::RunLabelDeferred), call, String::new())
                }));
                rows
            }
        }
    }
}

struct Row {
    label: Label,
    text: String,
    trailing: String,
}

enum Label {
    Kind(String),
    Speaker(String),
    Unspoken,
    Choice { number: usize, selected: bool },
}

impl Row {
    fn kind(label: String, text: &str, trailing: String) -> Self {
        Self {
            label: Label::Kind(label),
            text: text.to_owned(),
            trailing,
        }
    }

    fn line(line: &RunLine) -> Self {
        Self {
            label: line.speaker.as_deref().map_or(Label::Unspoken, |speaker| {
                Label::Speaker(speaker.to_uppercase())
            }),
            text: line.text.clone(),
            trailing: line.id.clone(),
        }
    }
}

struct Layout {
    label: usize,
    columns: Option<usize>,
}

impl Layout {
    fn new(label: usize, columns: Option<usize>) -> Self {
        Self {
            label,
            columns: columns.map(|columns| columns.min(MAX_COLUMNS)),
        }
    }

    fn write(&self, writer: &mut dyn Write, ink: Ink, row: &Row) -> Result<(), CliError> {
        let (label_text, label) = self.label(ink, &row.label);
        let pad = " ".repeat(self.label.saturating_sub(label_text.width()) + GAP);
        let indent = " ".repeat(self.label + GAP);
        let trailing = if row.trailing.is_empty() {
            String::new()
        } else {
            ink.teal(&row.trailing)
        };
        let Some(columns) = self.columns else {
            let separator = if row.trailing.is_empty() { "" } else { "  " };
            let line = format!("{label}{pad}{}{separator}{trailing}", row.text);
            writeln!(writer, "{}", line.trim_end())?;
            return Ok(());
        };
        // Each row keeps the terminal's width for its words and sets its own ID or result at the
        // right edge; one too wide to leave a usable measure follows its words instead.
        let room = columns.saturating_sub(self.label + GAP);
        if !row.trailing.is_empty() && room.saturating_sub(row.trailing.width() + GAP) < MIN_MEASURE
        {
            let joined = Row {
                label: Label::Kind(label_text),
                text: format!("{} · {}", row.text, row.trailing),
                trailing: String::new(),
            };
            return self.write(writer, ink, &joined);
        }
        let trailing_width = row.trailing.width();
        let measure = if row.trailing.is_empty() {
            room
        } else {
            room.saturating_sub(trailing_width + GAP)
        }
        .max(MIN_MEASURE);
        let wrapped = if row.text.is_empty() {
            vec![std::borrow::Cow::Borrowed("")]
        } else {
            textwrap::wrap(
                &row.text,
                textwrap::Options::new(measure).break_words(false),
            )
        };
        for (index, text) in wrapped.iter().enumerate() {
            if index == 0 {
                let fill = if row.trailing.is_empty() {
                    0
                } else {
                    measure.saturating_sub(text.width()) + GAP
                };
                let line = format!("{label}{pad}{text}{}{trailing}", " ".repeat(fill));
                writeln!(writer, "{}", line.trim_end())?;
            } else {
                writeln!(writer, "{indent}{text}")?;
            }
        }
        Ok(())
    }

    /// The label's plain text, for measuring, and its inked form.
    fn label(&self, ink: Ink, label: &Label) -> (String, String) {
        match label {
            Label::Kind(text) => (text.clone(), ink.teal(text)),
            Label::Speaker(text) => {
                let text = truncate(text, self.label);
                (text.clone(), ink.plum(&text))
            }
            Label::Unspoken => (String::new(), String::new()),
            Label::Choice { number, selected } => {
                let marker = if *selected { ">" } else { " " };
                let text = format!("{marker} {number:>2}");
                let text = format!("{text:>width$}", width = self.label);
                let inked = if *selected {
                    ink.plum(&text)
                } else {
                    text.clone()
                };
                (text, inked)
            }
        }
    }
}

fn truncate(text: &str, width: usize) -> String {
    if text.width() <= width {
        return text.to_owned();
    }
    let mut kept = String::new();
    for character in text.chars() {
        if kept.width() + 2 > width {
            break;
        }
        kept.push(character);
    }
    kept.push('…');
    kept
}

#[cfg(test)]
mod tests;
