pub(crate) mod project;

mod conditions;
mod effects;
mod ids;
mod markup;
mod metadata;
mod metadata_domains;
mod participation;
mod spans;
pub(crate) mod state;
mod statements;
mod values;

use recite_core::{Diagnostic, ProjectSchema, SourceFile};

pub use self::participation::{ValidationCompleteness, ValidationInput, ValidationParticipation};
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

/// Validate paired source-file summaries as one project.
///
/// The caller supplies explicit completeness for each recoverable summary
/// class. Incomplete classes do not contribute speculative diagnostics or
/// project-wide index evidence.
#[must_use]
pub fn validate_source_files_with_participation(
    source_files: &[ValidationInput<'_>],
) -> ValidationReport {
    validate_source_files_with_participation_and_optional_schema(
        source_files.iter().copied(),
        None,
        true,
    )
}

/// Validate recoverable source-file summaries when the project input set is
/// incomplete. File-local checks still run, while checks that require a
/// complete project index are left indeterminate.
#[must_use]
pub fn validate_source_files_with_incomplete_project(
    source_files: &[ValidationInput<'_>],
) -> ValidationReport {
    validate_source_files_with_participation_and_optional_schema(
        source_files.iter().copied(),
        None,
        false,
    )
}

/// Validate paired source-file summaries against a loaded project schema.
#[must_use]
pub fn validate_source_files_with_participation_with_schema(
    source_files: &[ValidationInput<'_>],
    schema: &ProjectSchema,
) -> ValidationReport {
    validate_source_files_with_participation_and_optional_schema(
        source_files.iter().copied(),
        Some(schema),
        true,
    )
}

/// Validate recoverable source-file summaries against a schema when the
/// project input set is incomplete.
#[must_use]
pub fn validate_source_files_with_incomplete_project_with_schema(
    source_files: &[ValidationInput<'_>],
    schema: &ProjectSchema,
) -> ValidationReport {
    validate_source_files_with_participation_and_optional_schema(
        source_files.iter().copied(),
        Some(schema),
        false,
    )
}

fn validate_source_files_with_optional_schema(
    source_files: &[SourceFile],
    schema: Option<&ProjectSchema>,
) -> ValidationReport {
    validate_source_files_with_participation_and_optional_schema(
        source_files.iter().map(ValidationInput::all_complete),
        schema,
        true,
    )
}

fn validate_source_files_with_participation_and_optional_schema<'a>(
    source_files: impl IntoIterator<Item = ValidationInput<'a>>,
    schema: Option<&'a ProjectSchema>,
    project_complete: bool,
) -> ValidationReport {
    let mut validator = Validator::new(source_files, schema, project_complete);
    validator.validate();
    sort_diagnostics_by_source(&mut validator.diagnostics);

    ValidationReport {
        diagnostics: validator.diagnostics,
    }
}
