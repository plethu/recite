use recite_core::SourceFile;

/// Whether a source-file summary is complete enough for one validation class.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidationCompleteness {
    /// The summary contains all information needed for this validation class.
    Complete,
    /// The summary may be useful for recovery, but must not contribute to this
    /// validation class or its project-wide index.
    Incomplete,
}

/// Short alias for [`ValidationCompleteness`] when the surrounding API makes
/// the validation context clear.
pub type Participation = ValidationCompleteness;

impl ValidationCompleteness {
    #[must_use]
    pub const fn is_complete(self) -> bool {
        matches!(self, Self::Complete)
    }
}

/// A source file's participation in compiler validation and project indexes.
///
/// Each class is independent because an editor may recover one part of a
/// malformed document while another part remains incomplete. Incomplete
/// classes are suppressed at the validation boundary rather than being
/// validated and filtered from the resulting diagnostics.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidationParticipation {
    pub ast_structure: ValidationCompleteness,
    pub block_definitions: ValidationCompleteness,
    pub block_references: ValidationCompleteness,
    pub stable_ids: ValidationCompleteness,
    pub metadata: ValidationCompleteness,
    pub condition_functions: ValidationCompleteness,
    pub effect_functions: ValidationCompleteness,
    pub inline_markup: ValidationCompleteness,
}

impl ValidationParticipation {
    /// Participation for a fully parsed and lowered source file.
    #[must_use]
    pub const fn all_complete() -> Self {
        Self {
            ast_structure: ValidationCompleteness::Complete,
            block_definitions: ValidationCompleteness::Complete,
            block_references: ValidationCompleteness::Complete,
            stable_ids: ValidationCompleteness::Complete,
            metadata: ValidationCompleteness::Complete,
            condition_functions: ValidationCompleteness::Complete,
            effect_functions: ValidationCompleteness::Complete,
            inline_markup: ValidationCompleteness::Complete,
        }
    }

    /// Alias for [`Self::all_complete`] for callers constructing legacy-style
    /// complete validation inputs.
    #[must_use]
    pub const fn complete() -> Self {
        Self::all_complete()
    }
}

impl Default for ValidationParticipation {
    fn default() -> Self {
        Self::all_complete()
    }
}

/// A borrowed source file paired with the completeness of its recoverable
/// compiler summary.
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ValidationSourceFile<'a> {
    pub source_file: &'a SourceFile,
    pub participation: ValidationParticipation,
}

impl<'a> ValidationSourceFile<'a> {
    #[must_use]
    pub const fn new(source_file: &'a SourceFile, participation: ValidationParticipation) -> Self {
        Self {
            source_file,
            participation,
        }
    }

    #[must_use]
    pub const fn all_complete(source_file: &'a SourceFile) -> Self {
        Self::new(source_file, ValidationParticipation::all_complete())
    }
}

/// Short alias for the paired validation input.
pub type ValidationInput<'a> = ValidationSourceFile<'a>;

/// Descriptive alias for callers that prefer the source-file-oriented name.
pub type SourceFileValidationInput<'a> = ValidationSourceFile<'a>;
