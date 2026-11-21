//! Human diagnostic layout: a rustc-like block per diagnostic and a closing count.
//! Structured output never passes through here and keeps its own records.
use std::collections::BTreeSet;
use std::io::Write;
use std::path::Path;

use recite_core::{DiagnosticRecord, DiagnosticSeverity, SourceSpan};
use recite_ui::RenderedDiagnostic;

use crate::error::CliError;
use crate::i18n::{Messages, MsgId};
use crate::ink::Ink;

pub(super) type Paint = Ink;

pub(super) fn write_diagnostic(
    writer: &mut dyn Write,
    paint: Paint,
    record: &DiagnosticRecord,
    rendered: &RenderedDiagnostic,
    source_line: Option<&str>,
) -> Result<(), CliError> {
    let line = record.span.start.line();
    let gutter = " ".repeat(line.to_string().len());
    write_headline(writer, paint, record, &rendered.primary_text)?;
    writeln!(
        writer,
        "{gutter} {} {}",
        paint.teal("-->"),
        paint.path(location(&record.span))
    )?;
    if let Some((text, start, width)) = source_line.and_then(|text| excerpt(&record.span, text)) {
        writeln!(writer, "{gutter} {}", paint.teal("|"))?;
        writeln!(writer, "{} {} {text}", paint.teal(line), paint.teal("|"))?;
        writeln!(
            writer,
            "{gutter} {} {}{}",
            paint.teal("|"),
            " ".repeat(start),
            paint.plum("^".repeat(width))
        )?;
    }
    for related in &rendered.related {
        writeln!(
            writer,
            "{gutter} {} {} ({})",
            paint.teal("= related:"),
            related.text,
            paint.path(location(&related.span))
        )?;
    }
    if let Some(help) = &rendered.help {
        writeln!(writer, "{gutter} {} {help}", paint.teal_strong("= help:"))?;
    }
    writeln!(writer)?;
    Ok(())
}

/// `error[CODE]: message`, with the message in bold. On a terminal a long message wraps at word
/// boundaries under its own first word, so paths and quoted names are never split.
fn write_headline(
    writer: &mut dyn Write,
    paint: Paint,
    record: &DiagnosticRecord,
    message: &str,
) -> Result<(), CliError> {
    let code = format!("[{}]", record.code.as_str());
    let prefix = severity_name(record.severity).len() + code.len() + 2;
    let rows = match crate::ink::stderr_columns() {
        Some(columns) if columns > prefix + 20 => textwrap::wrap(
            message,
            textwrap::Options::new(columns - prefix).break_words(false),
        ),
        _ => vec![std::borrow::Cow::Borrowed(message)],
    };
    let indent = " ".repeat(prefix);
    for (index, row) in rows.iter().enumerate() {
        if index == 0 {
            writeln!(
                writer,
                "{}{code}{} {}",
                severity(paint, record.severity),
                paint.strong(":"),
                paint.strong(row)
            )?;
        } else {
            writeln!(writer, "{indent}{}", paint.strong(row))?;
        }
    }
    Ok(())
}

fn severity(paint: Paint, severity: DiagnosticSeverity) -> String {
    let name = severity_name(severity);
    match severity {
        DiagnosticSeverity::Error => paint.error(name),
        DiagnosticSeverity::Warning => paint.warning(name),
        DiagnosticSeverity::Information | DiagnosticSeverity::Hint => paint.strong(name),
    }
}

/// Counts by severity and the distinct files they came from.
#[derive(Default)]
pub(super) struct Summary {
    errors: usize,
    warnings: usize,
    notes: usize,
    files: BTreeSet<String>,
}

impl Summary {
    pub(super) fn count(&mut self, severity: DiagnosticSeverity, file: &str) {
        match severity {
            DiagnosticSeverity::Error => self.errors += 1,
            DiagnosticSeverity::Warning => self.warnings += 1,
            DiagnosticSeverity::Information | DiagnosticSeverity::Hint => self.notes += 1,
        }
        self.files.insert(file.to_owned());
    }

    pub(super) fn total(&self) -> usize {
        self.errors + self.warnings + self.notes
    }

    pub(super) fn write(
        &self,
        writer: &mut dyn Write,
        paint: Paint,
        messages: &Messages,
    ) -> Result<(), CliError> {
        if self.total() == 0 {
            return Ok(());
        }
        let problems = [
            (self.errors, MsgId::CliDiagnosticErrors),
            (self.warnings, MsgId::CliDiagnosticWarnings),
            (self.notes, MsgId::CliDiagnosticNotes),
        ]
        .into_iter()
        .filter(|(count, _)| *count > 0)
        .map(|(count, id)| messages.format(id, [("count", count)]))
        .collect::<Vec<_>>()
        .join(", ");
        let line = messages.format(
            MsgId::CliDiagnosticSummary,
            [
                ("problems", recite_ui::UiArg::from(problems)),
                ("files", recite_ui::UiArg::from(self.files.len())),
            ],
        );
        writeln!(writer, "{}", paint.strong(line))?;
        Ok(())
    }
}

/// `ok  3 files, no diagnostics`: the human success line, on stderr beside the diagnostics.
pub(crate) fn report_success(
    writer: &mut dyn Write,
    messages: &Messages,
    subject: Subject<'_>,
) -> Result<(), CliError> {
    let paint = Paint::stderr();
    let rest = match subject {
        Subject::Files(count) => messages.format(MsgId::CliSuccessFiles, [("count", count)]),
        Subject::Project => messages.text(MsgId::CliSuccessProject),
        Subject::Compiled { files, output } => messages.format(
            MsgId::CliSuccessCompiled,
            [
                ("count", recite_ui::UiArg::from(files)),
                (
                    "output",
                    recite_ui::UiArg::from(display_path(&output.to_string_lossy())),
                ),
            ],
        ),
        Subject::Extracted { entries, output } => messages.format(
            MsgId::CliSuccessExtracted,
            [
                ("count", recite_ui::UiArg::from(entries)),
                (
                    "output",
                    recite_ui::UiArg::from(display_path(&output.to_string_lossy())),
                ),
            ],
        ),
        Subject::Message(text) => text,
    };
    writeln!(
        writer,
        "{}  {}",
        paint.teal_strong(messages.text(MsgId::CliSuccessOk)),
        rest
    )?;
    Ok(())
}

pub(crate) enum Subject<'a> {
    Files(usize),
    Project,
    /// Source files compiled into one asset.
    Compiled {
        files: usize,
        output: &'a Path,
    },
    /// POT entries written to a file.
    Extracted {
        entries: usize,
        output: &'a Path,
    },
    /// A command-specific sentence after the `ok` label.
    Message(String),
}

fn severity_name(severity: DiagnosticSeverity) -> &'static str {
    match severity {
        DiagnosticSeverity::Error => "error",
        DiagnosticSeverity::Warning => "warning",
        DiagnosticSeverity::Information => "info",
        DiagnosticSeverity::Hint => "hint",
    }
}

fn location(span: &SourceSpan) -> String {
    format!(
        "{}:{}:{}",
        display_path(&span.file),
        span.start.line(),
        span.start.column()
    )
}

/// Paths under the working directory print relative to it; anything else prints as given.
pub(crate) fn display_path(file: &str) -> String {
    let path = Path::new(file);
    std::env::current_dir()
        .ok()
        .and_then(|cwd| path.strip_prefix(cwd).ok().map(Path::to_path_buf))
        .map_or_else(
            || file.to_owned(),
            |relative| relative.display().to_string(),
        )
}

/// The span's source line, the caret's start column and its width, when the file is readable.
fn excerpt(span: &SourceSpan, text: &str) -> Option<(String, usize, usize)> {
    let start = usize::try_from(span.start.column()).ok()?.checked_sub(1)?;
    let length = text.chars().count();
    let width = span
        .end
        .filter(|end| end.line() == span.start.line())
        .and_then(|end| usize::try_from(end.column()).ok())
        .map_or_else(
            || unended_width(text, start),
            |end| end.saturating_sub(start + 1).max(1),
        );
    let width = width.min(length.saturating_sub(start).max(1));
    Some((text.replace('\t', " "), start.min(length), width))
}

/// A span with no end marks where something starts; underline the rest of that line's content so
/// the reader sees the whole reference or header rather than its first character.
fn unended_width(text: &str, start: usize) -> usize {
    text.chars()
        .skip(start)
        .collect::<String>()
        .trim_end()
        .chars()
        .count()
        .max(1)
}
