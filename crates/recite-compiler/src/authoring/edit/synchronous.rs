use super::StableIdTarget;
use super::{AuthoringEditError, AuthoringEditPlan};
use crate::authoring::CancellationToken;
use crate::authoring::{AuthoringSnapshot, SourceRange};
use recite_core::{DocumentKey, SourcePosition, StableIdGenerator};
/// Synchronous edit planning against a completed snapshot.
pub fn plan_rename_stable_id(
    snapshot: &AuthoringSnapshot,
    target: &StableIdTarget,
    new_id: &str,
) -> Result<AuthoringEditPlan, AuthoringEditError> {
    snapshot
        .query(&CancellationToken::new())
        .plan_rename_stable_id(target, new_id)
}
/// Synchronous edit planning against a completed snapshot.
pub fn plan_reassign_stable_id(
    snapshot: &AuthoringSnapshot,
    target: &StableIdTarget,
    ids: &mut StableIdGenerator,
) -> Result<AuthoringEditPlan, AuthoringEditError> {
    snapshot
        .query(&CancellationToken::new())
        .plan_reassign_stable_id(target, ids)
}
/// Synchronous edit planning against a completed snapshot.
pub fn plan_rename_block(
    snapshot: &AuthoringSnapshot,
    key: &DocumentKey,
    position: SourcePosition,
    new_name: &str,
) -> Result<AuthoringEditPlan, AuthoringEditError> {
    snapshot
        .query(&CancellationToken::new())
        .plan_rename_block(key, position, new_name)
}
/// Synchronous edit planning against a completed snapshot.
pub fn plan_insert_missing_ids(
    snapshot: &AuthoringSnapshot,
    ids: &mut StableIdGenerator,
) -> Result<AuthoringEditPlan, AuthoringEditError> {
    snapshot
        .query(&CancellationToken::new())
        .plan_insert_missing_ids(ids)
}
/// Synchronous edit planning against a completed snapshot.
pub fn plan_insert_missing_id(
    snapshot: &AuthoringSnapshot,
    key: &recite_core::DocumentKey,
    position: SourcePosition,
    ids: &mut StableIdGenerator,
) -> Result<AuthoringEditPlan, AuthoringEditError> {
    snapshot
        .query(&CancellationToken::new())
        .plan_insert_missing_id(key, position, ids)
}
/// Synchronous edit planning against a completed snapshot.
pub fn plan_insert_missing_ids_for_document(
    snapshot: &AuthoringSnapshot,
    key: &recite_core::DocumentKey,
    ids: &mut StableIdGenerator,
) -> Result<AuthoringEditPlan, AuthoringEditError> {
    snapshot
        .query(&CancellationToken::new())
        .plan_insert_missing_ids_for_document(key, ids)
}
/// Synchronous edit planning against a completed snapshot.
pub fn plan_insert_missing_ids_in_range(
    snapshot: &AuthoringSnapshot,
    key: &recite_core::DocumentKey,
    range: SourceRange,
    ids: &mut StableIdGenerator,
) -> Result<AuthoringEditPlan, AuthoringEditError> {
    snapshot
        .query(&CancellationToken::new())
        .plan_insert_missing_ids_in_range(key, range, ids)
}
/// Synchronous edit planning against a completed snapshot.
pub fn plan_create_block_stub(
    snapshot: &AuthoringSnapshot,
    key: &DocumentKey,
    position: SourcePosition,
) -> Result<AuthoringEditPlan, AuthoringEditError> {
    snapshot
        .query(&CancellationToken::new())
        .plan_create_block_stub(key, position)
}
/// Synchronous edit planning against a completed snapshot.
pub fn plan_create_block_stub_in_range(
    snapshot: &AuthoringSnapshot,
    key: &DocumentKey,
    range: SourceRange,
) -> Result<AuthoringEditPlan, AuthoringEditError> {
    snapshot
        .query(&CancellationToken::new())
        .plan_create_block_stub_in_range(key, range)
}
impl AuthoringSnapshot {
    pub fn plan_rename_stable_id(
        &self,
        target: &StableIdTarget,
        new_id: &str,
    ) -> Result<AuthoringEditPlan, AuthoringEditError> {
        plan_rename_stable_id(self, target, new_id)
    }
    pub fn plan_reassign_stable_id(
        &self,
        target: &StableIdTarget,
        ids: &mut StableIdGenerator,
    ) -> Result<AuthoringEditPlan, AuthoringEditError> {
        plan_reassign_stable_id(self, target, ids)
    }
    pub fn plan_rename_block(
        &self,
        key: &DocumentKey,
        position: SourcePosition,
        new_name: &str,
    ) -> Result<AuthoringEditPlan, AuthoringEditError> {
        plan_rename_block(self, key, position, new_name)
    }
    pub fn plan_insert_missing_ids(
        &self,
        ids: &mut StableIdGenerator,
    ) -> Result<AuthoringEditPlan, AuthoringEditError> {
        plan_insert_missing_ids(self, ids)
    }
    pub fn plan_insert_missing_id(
        &self,
        key: &recite_core::DocumentKey,
        position: SourcePosition,
        ids: &mut StableIdGenerator,
    ) -> Result<AuthoringEditPlan, AuthoringEditError> {
        plan_insert_missing_id(self, key, position, ids)
    }
    pub fn plan_insert_missing_ids_for_document(
        &self,
        key: &recite_core::DocumentKey,
        ids: &mut StableIdGenerator,
    ) -> Result<AuthoringEditPlan, AuthoringEditError> {
        plan_insert_missing_ids_for_document(self, key, ids)
    }
    pub fn plan_insert_missing_ids_in_range(
        &self,
        key: &recite_core::DocumentKey,
        range: SourceRange,
        ids: &mut StableIdGenerator,
    ) -> Result<AuthoringEditPlan, AuthoringEditError> {
        plan_insert_missing_ids_in_range(self, key, range, ids)
    }
    pub fn plan_create_block_stub(
        &self,
        key: &DocumentKey,
        position: SourcePosition,
    ) -> Result<AuthoringEditPlan, AuthoringEditError> {
        plan_create_block_stub(self, key, position)
    }
    pub fn plan_create_block_stub_in_range(
        &self,
        key: &DocumentKey,
        range: SourceRange,
    ) -> Result<AuthoringEditPlan, AuthoringEditError> {
        plan_create_block_stub_in_range(self, key, range)
    }
}
