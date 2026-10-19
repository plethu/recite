//! Human diagnostic layout: a rustc-like block per diagnostic and a closing count.
//! Structured output never passes through here and keeps its own records.
use std::collections::BTreeSet;
use std::io::Write;
use std::path::Path;
use std::sync::atomic::{AtomicBool, Ordering};

use recite_core::{DiagnosticRecord, DiagnosticSeverity, SourceSpan};
use recite_ui::RenderedDiagnostic;

use crate::error::CliError;
use crate::i18n::{Messages, MsgId};

/// Set once by the binary from the stderr terminal check and the shared colour settings.
/// Library callers and tests keep the plain default.
static COLOR: AtomicBool = AtomicBool::new(false);

pub(crate) fn set_color(enabled: bool) {
    COLOR.store(enabled, Ordering::Relaxed);
}

#[derive(Clone, Copy)]
pub(super) struct Paint {
    color: bool,
}

impl Paint {
    pub(super) fn current() -> Self {
        Self {
            color: COLOR.load(Ordering::Relaxed),
        }
    }

    fn sgr(self, code: &str, text: &str) -> String {
        if self.color {
            format!("\u{1b}[{code}m{text}\u{1b}[0m")
        } else {
            text.to_owned()
        }
    }

    fn severity(self, severity: DiagnosticSeverity, text: &str) -> String {
        self.sgr(
            match severity {
                DiagnosticSeverity::Error => "1;31",
                DiagnosticSeverity::Warning => "1;33",
                DiagnosticSeverity::Information | DiagnosticSeverity::Hint => "1",
            },
            text,
        )
    }

    fn bold(self, text: &str) -> String {
        self.sgr("1", text)
    }

    fn dim(self, text: &str) -> String {
        self.sgr("2", text)
    }

    fn path(self, text: &str) -> String {
        self.sgr("4", text)
    }

    fn caret(self, text: &str) -> String {
        self.sgr("1;35", text)
    }
}

pub(super) fn write_diagnostic(
    writer: &mut dyn Write,
    paint: Paint,
    record: &DiagnosticRecord,
    rendered: &RenderedDiagnostic,
    source_line: Option<&str>,
) -> Result<(), CliError> {
    let line = record.span.start.line();
    let gutter = " ".repeat(line.to_string().len());
    writeln!(
        writer,
        "{}{}{} {}",
        paint.severity(record.severity, severity_name(record.severity)),
        paint.dim(&format!("[{}]", record.code.as_str())),
        paint.bold(":"),
        rendered.primary_text
    )?;
    writeln!(
        writer,
        "{gutter} {} {}",
        paint.dim("-->"),
        paint.path(&location(&record.span))
    )?;
    if let Some((text, start, width)) = source_line.and_then(|text| excerpt(&record.span, text)) {
        writeln!(writer, "{gutter} {}", paint.dim("|"))?;
        writeln!(
            writer,
            "{} {} {text}",
            paint.dim(&line.to_string()),
            paint.dim("|")
        )?;
        writeln!(
            writer,
            "{gutter} {} {}{}",
            paint.dim("|"),
            " ".repeat(start),
            paint.caret(&"^".repeat(width))
        )?;
    }
    for related in &rendered.related {
        writeln!(
            writer,
            "{gutter} {} {} ({})",
            paint.dim("= related:"),
            related.text,
            paint.path(&location(&related.span))
        )?;
    }
    if let Some(help) = &rendered.help {
        writeln!(writer, "{gutter} {} {help}", paint.dim("= help:"))?;
    }
    writeln!(writer)?;
    Ok(())
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
        writeln!(writer, "{}", paint.bold(&line))?;
        Ok(())
    }
}

/// `ok  3 files, no diagnostics`: the human success line, on stderr beside the diagnostics.
pub(crate) fn report_success(
    writer: &mut dyn Write,
    messages: &Messages,
    subject: Subject,
) -> Result<(), CliError> {
    let paint = Paint::current();
    let rest = match subject {
        Subject::Files(count) => messages.format(MsgId::CliSuccessFiles, [("count", count)]),
        Subject::Project => messages.text(MsgId::CliSuccessProject),
    };
    writeln!(
        writer,
        "{}  {}",
        paint.bold(&messages.text(MsgId::CliSuccessOk)),
        paint.dim(&rest)
    )?;
    Ok(())
}

#[derive(Clone, Copy)]
pub(crate) enum Subject {
    Files(usize),
    Project,
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
fn display_path(file: &str) -> String {
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
        .map_or(1, |end| end.saturating_sub(start + 1).max(1));
    let width = width.min(length.saturating_sub(start).max(1));
    Some((text.replace('\t', " "), start.min(length), width))
}
