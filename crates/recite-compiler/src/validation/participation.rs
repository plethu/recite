use std::collections::BTreeMap;

use recite_core::SourceFile;

/// Whether a source-file summary is complete enough for one validation class.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ValidationCompleteness {
    /// The summary contains all information needed for this validation class.
    Complete,
    /// The summary may be useful for recovery, but must not contribute to this
    /// validation class or its project-wide index.
    Incomplete,
}

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
#[non_exhaustive]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ValidationParticipation {
    ast_structure: ValidationCompleteness,
    block_definitions: ValidationCompleteness,
    block_references: ValidationCompleteness,
    stable_ids: ValidationCompleteness,
    metadata: ValidationCompleteness,
    condition_functions: ValidationCompleteness,
    effect_functions: ValidationCompleteness,
    inline_markup: ValidationCompleteness,
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

    /// Participation for a summary with no complete validation classes.
    #[must_use]
    pub const fn all_incomplete() -> Self {
        Self {
            ast_structure: ValidationCompleteness::Incomplete,
            block_definitions: ValidationCompleteness::Incomplete,
            block_references: ValidationCompleteness::Incomplete,
            stable_ids: ValidationCompleteness::Incomplete,
            metadata: ValidationCompleteness::Incomplete,
            condition_functions: ValidationCompleteness::Incomplete,
            effect_functions: ValidationCompleteness::Incomplete,
            inline_markup: ValidationCompleteness::Incomplete,
        }
    }

    #[must_use]
    pub const fn ast_structure(self) -> ValidationCompleteness {
        self.ast_structure
    }

    #[must_use]
    pub const fn block_definitions(self) -> ValidationCompleteness {
        self.block_definitions
    }

    #[must_use]
    pub const fn block_references(self) -> ValidationCompleteness {
        self.block_references
    }

    #[must_use]
    pub const fn stable_ids(self) -> ValidationCompleteness {
        self.stable_ids
    }

    #[must_use]
    pub const fn metadata(self) -> ValidationCompleteness {
        self.metadata
    }

    #[must_use]
    pub const fn condition_functions(self) -> ValidationCompleteness {
        self.condition_functions
    }

    #[must_use]
    pub const fn effect_functions(self) -> ValidationCompleteness {
        self.effect_functions
    }

    #[must_use]
    pub const fn inline_markup(self) -> ValidationCompleteness {
        self.inline_markup
    }

    #[must_use]
    pub const fn with_ast_structure(mut self, completeness: ValidationCompleteness) -> Self {
        self.ast_structure = completeness;
        self
    }

    #[must_use]
    pub const fn with_block_definitions(mut self, completeness: ValidationCompleteness) -> Self {
        self.block_definitions = completeness;
        self
    }

    #[must_use]
    pub const fn with_block_references(mut self, completeness: ValidationCompleteness) -> Self {
        self.block_references = completeness;
        self
    }

    #[must_use]
    pub const fn with_stable_ids(mut self, completeness: ValidationCompleteness) -> Self {
        self.stable_ids = completeness;
        self
    }

    #[must_use]
    pub const fn with_metadata(mut self, completeness: ValidationCompleteness) -> Self {
        self.metadata = completeness;
        self
    }

    #[must_use]
    pub const fn with_condition_functions(mut self, completeness: ValidationCompleteness) -> Self {
        self.condition_functions = completeness;
        self
    }

    #[must_use]
    pub const fn with_effect_functions(mut self, completeness: ValidationCompleteness) -> Self {
        self.effect_functions = completeness;
        self
    }

    #[must_use]
    pub const fn with_inline_markup(mut self, completeness: ValidationCompleteness) -> Self {
        self.inline_markup = completeness;
        self
    }

    pub(crate) const fn merge(self, other: Self) -> Self {
        Self {
            ast_structure: merge_completeness(self.ast_structure, other.ast_structure),
            block_definitions: merge_completeness(self.block_definitions, other.block_definitions),
            block_references: merge_completeness(self.block_references, other.block_references),
            stable_ids: merge_completeness(self.stable_ids, other.stable_ids),
            metadata: merge_completeness(self.metadata, other.metadata),
            condition_functions: merge_completeness(
                self.condition_functions,
                other.condition_functions,
            ),
            effect_functions: merge_completeness(self.effect_functions, other.effect_functions),
            inline_markup: merge_completeness(self.inline_markup, other.inline_markup),
        }
    }
}

const fn merge_completeness(
    left: ValidationCompleteness,
    right: ValidationCompleteness,
) -> ValidationCompleteness {
    if left.is_complete() && right.is_complete() {
        ValidationCompleteness::Complete
    } else {
        ValidationCompleteness::Incomplete
    }
}

impl Default for ValidationParticipation {
    fn default() -> Self {
        Self::all_complete()
    }
}

/// A borrowed source file paired with the completeness of its recoverable
/// compiler summary.
#[non_exhaustive]
#[derive(Clone, Copy, Debug, PartialEq)]
pub struct ValidationInput<'a> {
    source_file: &'a SourceFile,
    participation: ValidationParticipation,
}

impl<'a> ValidationInput<'a> {
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

    #[must_use]
    pub const fn source_file(&self) -> &'a SourceFile {
        self.source_file
    }

    #[must_use]
    pub const fn participation(&self) -> ValidationParticipation {
        self.participation
    }
}

pub(crate) fn aggregate_participation<'a>(
    source_files: &[ValidationInput<'a>],
) -> BTreeMap<&'a str, ValidationParticipation> {
    let mut effective = BTreeMap::new();
    for source_file in source_files {
        let path = source_file.source_file().path.as_str();
        effective
            .entry(path)
            .and_modify(|participation: &mut ValidationParticipation| {
                *participation = participation.merge(source_file.participation())
            })
            .or_insert(source_file.participation());
    }
    effective
}
