use recite_core::SourcePosition;

use super::helpers::document;
use super::plan_stable_ids::plan_insert;
use super::{AuthoringEditError, AuthoringEditPlan};
use crate::authoring::{AuthoringSnapshot, StableIdSummary};

/// Plans insertion of every missing or draft stable ID in one document.
///
/// The edits stay file-scoped, while the plan retains project-wide
/// preconditions so generated anchors and namespace collision checks remain
/// conditional on the complete snapshot used to plan them.
pub fn plan_insert_missing_ids_for_document(
    snapshot: &AuthoringSnapshot,
    key: &recite_core::DocumentKey,
) -> Result<AuthoringEditPlan, AuthoringEditError> {
    document(snapshot, key)?;
    plan_insert(snapshot, |candidate, _| candidate == key)
}

/// Plans insertion of stable IDs intersecting one source range in one
/// document. Candidate selection is summary-backed and does not walk source
/// characters; the resulting plan still validates the complete project.
pub fn plan_insert_missing_ids_in_range(
    snapshot: &AuthoringSnapshot,
    key: &recite_core::DocumentKey,
    range: super::SourceRange,
) -> Result<AuthoringEditPlan, AuthoringEditError> {
    document(snapshot, key)?;
    if range.start() > range.end() {
        return Err(AuthoringEditError::UnmappableRange {
            document: key.clone(),
        });
    }
    plan_insert(snapshot, |candidate, stable| {
        candidate == key && stable_selected_by_range(stable, range)
    })
}

impl AuthoringSnapshot {
    /// Plans insertion of all missing stable IDs in one document.
    pub fn plan_insert_missing_ids_for_document(
        &self,
        key: &recite_core::DocumentKey,
    ) -> Result<AuthoringEditPlan, AuthoringEditError> {
        plan_insert_missing_ids_for_document(self, key)
    }

    /// Plans insertion of stable IDs intersecting one source range.
    pub fn plan_insert_missing_ids_in_range(
        &self,
        key: &recite_core::DocumentKey,
        range: super::SourceRange,
    ) -> Result<AuthoringEditPlan, AuthoringEditError> {
        plan_insert_missing_ids_in_range(self, key, range)
    }
}

fn stable_selected_by_range(stable: &StableIdSummary, range: super::SourceRange) -> bool {
    if range.start() == range.end() {
        return stable_selected_at(stable, range.start());
    }
    stable.source_id_span().is_some_and(|span| {
        span.end
            .is_some_and(|end| range.start() <= end && range.end() > span.start)
    }) || std::iter::once(stable.span().start)
        .chain(stable.insertion_span().map(|span| span.start))
        .any(|point| range.start() <= point && point < range.end())
}

fn stable_selected_at(stable: &StableIdSummary, position: SourcePosition) -> bool {
    stable.source_id_span().is_some_and(|span| {
        span.end
            .is_some_and(|end| span.start <= position && position <= end)
    }) || stable.span().start == position
        || stable
            .insertion_span()
            .is_some_and(|span| span.start == position)
}
