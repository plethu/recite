use std::io::Write;

use recite_core::Diagnostic;

use crate::error::CliError;

pub(crate) fn report_diagnostics<'a>(
    writer: &mut dyn Write,
    diagnostics: impl Iterator<Item = &'a Diagnostic>,
) -> Result<usize, CliError> {
    let mut count = 0;
    for diagnostic in diagnostics {
        count += 1;
        writeln!(
            writer,
            "{} {} {}:{}:{} {}",
            severity_name(diagnostic.severity),
            diagnostic.code.as_str(),
            diagnostic.span.file,
            diagnostic.span.start.line(),
            diagnostic.span.start.column(),
            diagnostic.message
        )?;
        for related in &diagnostic.related {
            writeln!(
                writer,
                "  related {}:{}:{} {}",
                related.span.file,
                related.span.start.line(),
                related.span.start.column(),
                related.message
            )?;
        }
        if let Some(help) = &diagnostic.help {
            writeln!(writer, "  help: {help}")?;
        }
    }
    Ok(count)
}

pub(crate) struct InputDiagnostics {
    pub(crate) parse_diagnostics: Vec<Diagnostic>,
    pub(crate) validation_diagnostics: Vec<Diagnostic>,
}

impl InputDiagnostics {
    pub(crate) fn into_all(self) -> Vec<Diagnostic> {
        self.parse_diagnostics
            .into_iter()
            .chain(self.validation_diagnostics)
            .collect()
    }
}

pub(crate) fn report_targeted_diagnostics(
    writer: &mut dyn Write,
    diagnostics: InputDiagnostics,
    is_target: impl Fn(&Diagnostic) -> bool,
) -> Result<(), CliError> {
    if !diagnostics.parse_diagnostics.is_empty() {
        report_diagnostics(writer, diagnostics.parse_diagnostics.iter())?;
        return Err(CliError::Diagnostics);
    }

    let targeted = diagnostics
        .validation_diagnostics
        .iter()
        .filter(|diagnostic| is_target(diagnostic))
        .collect::<Vec<_>>();
    if targeted.is_empty() {
        return Ok(());
    }

    report_diagnostics(writer, targeted.into_iter())?;
    Err(CliError::Diagnostics)
}

fn severity_name(severity: recite_core::DiagnosticSeverity) -> &'static str {
    match severity {
        recite_core::DiagnosticSeverity::Error => "error",
        recite_core::DiagnosticSeverity::Warning => "warning",
        recite_core::DiagnosticSeverity::Information => "info",
        recite_core::DiagnosticSeverity::Hint => "hint",
    }
}
