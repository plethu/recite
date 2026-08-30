use super::super::{AuthoringSnapshot, SnapshotGeneration};
use super::error::AuthoringEditError;
use super::operation::AuthoringEditOperation;
use super::precondition::EditPrecondition;
use super::range::SourceEdit;

/// A deterministic, conditional set of source replacements.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct AuthoringEditPlan {
    expected_generation: SnapshotGeneration,
    preconditions: Vec<EditPrecondition>,
    edits: Vec<SourceEdit>,
    operation: AuthoringEditOperation,
}

impl AuthoringEditPlan {
    pub(crate) fn new(
        expected_generation: SnapshotGeneration,
        mut preconditions: Vec<EditPrecondition>,
        mut edits: Vec<SourceEdit>,
        operation: AuthoringEditOperation,
    ) -> Result<Self, AuthoringEditError> {
        preconditions.sort_by(|left, right| left.document().cmp(right.document()));
        edits.sort_by(|left, right| {
            left.document()
                .cmp(right.document())
                .then_with(|| left.range().cmp(&right.range()))
                .then_with(|| left.replacement().cmp(right.replacement()))
        });
        let plan = Self {
            expected_generation,
            preconditions,
            edits,
            operation,
        };
        plan.check_shape()?;
        Ok(plan)
    }

    #[must_use]
    pub const fn expected_generation(&self) -> SnapshotGeneration {
        self.expected_generation
    }

    #[must_use]
    pub fn preconditions(&self) -> &[EditPrecondition] {
        &self.preconditions
    }

    #[must_use]
    pub fn edits(&self) -> &[SourceEdit] {
        &self.edits
    }

    #[must_use]
    pub const fn operation(&self) -> &AuthoringEditOperation {
        &self.operation
    }

    /// Checks that this plan still applies to the supplied compiler snapshot.
    /// Hosts should perform this check before projecting or applying edits.
    pub fn validate(&self, snapshot: &AuthoringSnapshot) -> Result<(), AuthoringEditError> {
        if self.expected_generation != snapshot.generation() {
            return Err(AuthoringEditError::StaleGeneration {
                expected: self.expected_generation,
                actual: snapshot.generation(),
            });
        }
        self.check_shape()?;
        for precondition in &self.preconditions {
            let Some(document) = snapshot.document(precondition.document()) else {
                return Err(AuthoringEditError::StaleDocument {
                    document: precondition.document().clone(),
                });
            };
            if document.version() != precondition.expected_version() {
                return Err(AuthoringEditError::StaleDocumentVersion {
                    document: precondition.document().clone(),
                    expected: precondition.expected_version(),
                    actual: document.version(),
                });
            }
            if !precondition
                .source_fingerprint()
                .matches_source(document.source_text())
            {
                return Err(AuthoringEditError::StaleSource {
                    document: precondition.document().clone(),
                });
            }
        }
        for edit in &self.edits {
            let Some(document) = snapshot.document(edit.document()) else {
                return Err(AuthoringEditError::StaleDocument {
                    document: edit.document().clone(),
                });
            };
            super::validate::byte_offsets(document.source_text(), edit.range()).map_err(|_| {
                AuthoringEditError::UnmappableRange {
                    document: edit.document().clone(),
                }
            })?;
        }
        Ok(())
    }

    fn check_shape(&self) -> Result<(), AuthoringEditError> {
        if self.edits.is_empty() || self.preconditions.is_empty() {
            return Err(AuthoringEditError::NoEdits);
        }
        for pair in self.preconditions.windows(2) {
            if pair[0].document() == pair[1].document() {
                return Err(AuthoringEditError::DuplicatePrecondition {
                    document: pair[0].document().clone(),
                });
            }
        }
        for edit in &self.edits {
            if self
                .preconditions
                .binary_search_by(|precondition| precondition.document().cmp(edit.document()))
                .is_err()
            {
                return Err(AuthoringEditError::MissingPrecondition {
                    document: edit.document().clone(),
                });
            }
        }
        for pair in self.edits.windows(2) {
            if pair[0].document() == pair[1].document()
                && ranges_collide(pair[0].range(), pair[1].range())
            {
                return Err(AuthoringEditError::OverlappingEdits {
                    document: pair[0].document().clone(),
                });
            }
        }
        Ok(())
    }
}

fn ranges_collide(left: super::range::SourceRange, right: super::range::SourceRange) -> bool {
    let left_is_point = left.start() == left.end();
    let right_is_point = right.start() == right.end();
    left == right
        || (left.end() > right.start() && right.end() > left.start())
        || (left_is_point && left.start() >= right.start() && left.start() < right.end())
        || (right_is_point && right.start() >= left.start() && right.start() < left.end())
}
