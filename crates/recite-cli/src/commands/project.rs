use std::io::Write;
use std::path::PathBuf;

use crate::diagnostics::report_diagnostics;
use crate::error::CliError;
use crate::i18n::Messages;

pub(super) fn project_check(
    project_root: PathBuf,
    validate: fn(PathBuf) -> Result<Vec<recite_core::Diagnostic>, CliError>,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<(), CliError> {
    let diagnostics = validate(project_root)?;
    report_diagnostics(stderr, messages, diagnostics.iter())?;
    diagnostics
        .iter()
        .all(|diagnostic| diagnostic.severity != recite_core::DiagnosticSeverity::Error)
        .then_some(())
        .ok_or(CliError::Diagnostics)
}
