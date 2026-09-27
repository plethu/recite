pub(crate) mod incremental;
pub(crate) mod project;

mod conditions;
mod effects;
mod ids;
mod localisable_ids;
mod markup;
mod metadata;
mod metadata_domains;
mod participation;
mod spans;
pub(crate) mod state;
mod statements;
mod values;

use std::borrow::Borrow;

use recite_core::{Diagnostic, ast::SourceFile, schema::ProjectSchema};

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

/// Whether all project documents were available for cross-document checks.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ProjectCompleteness {
    Complete,
    Incomplete,
}

/// Validate parsed source files as one project.
#[must_use]
pub fn validate_source_files(source_files: &[SourceFile]) -> ValidationReport {
    validate_inputs(
        source_files.iter().map(ValidationInput::all_complete),
        None,
        ProjectCompleteness::Complete,
    )
}

/// Validate source files with per-document participation and optional schema.
///
/// The caller supplies explicit completeness for each recoverable summary
/// class. Incomplete classes do not contribute speculative diagnostics or
/// project-wide index evidence. `ProjectCompleteness::Incomplete` also defers
/// checks that need the full set of project documents.
#[must_use]
pub fn validate_inputs<'a, I>(
    source_files: I,
    schema: Option<&'a ProjectSchema>,
    project: ProjectCompleteness,
) -> ValidationReport
where
    I: IntoIterator,
    I::Item: std::borrow::Borrow<ValidationInput<'a>>,
{
    let mut validator = Validator::new(
        source_files.into_iter().map(|input| *input.borrow()),
        schema,
        project == ProjectCompleteness::Complete,
    );
    validator.validate();
    sort_diagnostics_by_source(&mut validator.diagnostics);

    ValidationReport {
        diagnostics: validator.diagnostics,
    }
}
