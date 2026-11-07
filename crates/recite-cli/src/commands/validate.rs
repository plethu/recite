//! `validate`: source validation, optionally against a schema and focused on one category.

use std::io::Write;

use crate::args::ValidateArgs;
use crate::diagnostics::{
    Subject, report_diagnostics, report_success, report_targeted_diagnostics,
};
use crate::error::CliError;
use crate::fs::{load_optional_schema, validate_inputs};
use crate::i18n::Messages;

pub(super) fn run(
    args: ValidateArgs,
    stderr: &mut dyn Write,
    messages: &Messages,
) -> Result<(), CliError> {
    let schema = load_optional_schema(args.schema.as_deref(), stderr, messages)?;
    let inputs = validate_inputs(&args.paths, schema.as_ref())?;
    if let Some(only) = args.only {
        return report_targeted_diagnostics(stderr, messages, inputs, only.category());
    }
    let file_count = inputs.file_count;
    let diagnostics = inputs.into_all();
    report_diagnostics(stderr, messages, diagnostics.iter())?;
    if !diagnostics.is_empty() {
        return Err(CliError::Diagnostics);
    }
    report_success(stderr, messages, Subject::Files(file_count))
}
