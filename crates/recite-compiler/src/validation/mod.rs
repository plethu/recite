pub(crate) mod project;

mod ids;
mod spans;
mod state;
mod statements;
mod values;

use recite_core::{Diagnostic, SourceFile};

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
    let mut validator = Validator::new(source_files);
    validator.validate();
    sort_diagnostics_by_source(&mut validator.diagnostics);

    ValidationReport {
        diagnostics: validator.diagnostics,
    }
}
