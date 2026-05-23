pub(crate) mod project;

mod effects;
mod ids;
mod markup;
mod metadata;
mod spans;
mod state;
mod statements;
mod values;

use recite_core::{Diagnostic, ProjectSchema, SourceFile};

use self::project::sort_diagnostics_by_source;
use self::state::Validator;

/// Result of semantic validation over one or more Recite source files.
#[derive(Clone, Debug, Default, PartialEq)]
pub struct ValidationReport {
    pub diagnostics: Vec<Diagnostic>,
}

impl ValidationReport {
    #[must_use]
    pub fn is_ok(&self) -> bool {
        self.diagnostics.is_empty()
    }
}

/// Validate one parsed source file.
#[must_use]
pub fn validate_source_file(source_file: &SourceFile) -> ValidationReport {
    validate_source_files(std::slice::from_ref(source_file))
}

/// Validate parsed source files as one project.
#[must_use]
pub fn validate_source_files(source_files: &[SourceFile]) -> ValidationReport {
    validate_source_files_with_optional_schema(source_files, None)
}

/// Validate parsed source files as one project against a loaded schema.
#[must_use]
pub fn validate_source_files_with_schema(
    source_files: &[SourceFile],
    schema: &ProjectSchema,
) -> ValidationReport {
    validate_source_files_with_optional_schema(source_files, Some(schema))
}

fn validate_source_files_with_optional_schema(
    source_files: &[SourceFile],
    schema: Option<&ProjectSchema>,
) -> ValidationReport {
    let mut validator = Validator::new(source_files, schema);
    validator.validate();
    sort_diagnostics_by_source(&mut validator.diagnostics);

    ValidationReport {
        diagnostics: validator.diagnostics,
    }
}
