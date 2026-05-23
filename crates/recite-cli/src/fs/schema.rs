use std::fs;
use std::io::Write;
use std::path::Path;

use recite_core::{Diagnostic, ProjectSchema, load_schema_manifest_str};

use super::paths::display_path;
use crate::diagnostics::report_diagnostics;
use crate::error::CliError;

pub(crate) struct LoadedSchema {
    pub(crate) schema: Option<ProjectSchema>,
    pub(crate) diagnostics: Vec<Diagnostic>,
}

pub(crate) fn load_optional_schema(
    schema_path: Option<&Path>,
    stderr: &mut dyn Write,
) -> Result<Option<ProjectSchema>, CliError> {
    let Some(schema_path) = schema_path else {
        return Ok(None);
    };

    let report = load_schema(schema_path)?;
    if !report.diagnostics.is_empty() {
        report_diagnostics(stderr, report.diagnostics.iter())?;
        return Err(CliError::Diagnostics);
    }

    Ok(report.schema)
}

pub(crate) fn load_schema(schema_path: &Path) -> Result<LoadedSchema, CliError> {
    let source = fs::read_to_string(schema_path).map_err(|source| CliError::Read {
        path: schema_path.to_owned(),
        source,
    })?;
    let report = load_schema_manifest_str(display_path(schema_path), &source);
    Ok(LoadedSchema {
        schema: report.schema,
        diagnostics: report.diagnostics,
    })
}
