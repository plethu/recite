use std::io::Write;
use std::path::PathBuf;

use crate::diagnostics::{Subject, report_diagnostics, report_success};
use crate::error::CliError;
use crate::i18n::Messages;

pub(super) fn project_check(
    project_root: PathBuf,
    validate: fn(PathBuf) -> Result<Vec<recite_core::Diagnostic>, CliError>,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<(), CliError> {
    let diagnostics = validate(project_root)?;
    let reported = report_diagnostics(stderr, messages, diagnostics.iter())?;
    if diagnostics
        .iter()
        .any(|diagnostic| diagnostic.severity == recite_core::DiagnosticSeverity::Error)
    {
        return Err(CliError::Diagnostics);
    }
    if reported == 0 {
        report_success(stderr, messages, Subject::Project)?;
    }
    Ok(())
}
