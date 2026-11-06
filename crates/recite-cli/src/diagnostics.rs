use std::io::Write;

use recite_core::{Diagnostic, DiagnosticCategory};

use crate::{error::CliError, i18n::Messages};

#[cfg(test)]
mod tests;

mod excerpts;
mod human;
pub(crate) use human::{Subject, report_success, set_color};

/// Render human diagnostics in a rustc-like layout, then a one-line count.
pub(crate) fn report_diagnostics<'a>(
    writer: &mut dyn Write,
    messages: &Messages,
    diagnostics: impl Iterator<Item = &'a Diagnostic>,
) -> Result<usize, CliError> {
    let paint = human::Paint::current();
    let mut summary = human::Summary::default();
    let diagnostics = diagnostics.collect::<Vec<_>>();
    let mut excerpts =
        excerpts::Excerpts::new(diagnostics.iter().map(|diagnostic| &diagnostic.span));
    for diagnostic in diagnostics {
        let record = diagnostic
            .record()
            .map_err(|source| CliError::DiagnosticRendering {
                source: source.to_string(),
            })?;
        let rendered = messages.render_diagnostic(&record).map_err(|source| {
            CliError::DiagnosticRendering {
                source: source.to_string(),
            }
        })?;
        summary.count(record.severity, &record.span.file);
        human::write_diagnostic(
            writer,
            paint,
            &record,
            &rendered,
            excerpts.line(&record.span),
        )?;
    }
    summary.write(writer, paint, messages)?;
    Ok(summary.total())
}

pub(crate) struct InputDiagnostics {
    pub(crate) parse_diagnostics: Vec<Diagnostic>,
    pub(crate) validation_diagnostics: Vec<Diagnostic>,
    /// How many source files were read, for the human success line.
    pub(crate) file_count: usize,
}

impl InputDiagnostics {
    pub(crate) fn into_all(self) -> Vec<Diagnostic> {
        self.parse_diagnostics
            .into_iter()
            .chain(self.validation_diagnostics)
            .collect()
    }
}

impl InputDiagnostics {
    /// Syntax errors, which block validation, or else the validation diagnostics in `category`.
    pub(crate) fn into_category(self, category: DiagnosticCategory) -> Vec<Diagnostic> {
        if !self.parse_diagnostics.is_empty() {
            return self.parse_diagnostics;
        }
        self.validation_diagnostics
            .into_iter()
            .filter(|diagnostic| diagnostic.code.category() == category)
            .collect()
    }
}

pub(crate) fn report_targeted_diagnostics(
    writer: &mut dyn Write,
    messages: &Messages,
    diagnostics: InputDiagnostics,
    category: DiagnosticCategory,
) -> Result<(), CliError> {
    if !diagnostics.parse_diagnostics.is_empty() {
        report_diagnostics(writer, messages, diagnostics.parse_diagnostics.iter())?;
        return Err(CliError::Diagnostics);
    }

    let targeted = diagnostics
        .validation_diagnostics
        .iter()
        .filter(|diagnostic| diagnostic.code.category() == category)
        .collect::<Vec<_>>();
    if targeted.is_empty() {
        return report_success(writer, messages, Subject::Files(diagnostics.file_count));
    }

    report_diagnostics(writer, messages, targeted.into_iter())?;
    Err(CliError::Diagnostics)
}
