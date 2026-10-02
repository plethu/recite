use super::{AuthoringEditError, AuthoringEditPlan};
use crate::authoring::CancellationToken;
use crate::authoring::{AuthoringSnapshot, SourceRange};
use recite_core::{DocumentKey, SourcePosition};
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
) -> Result<AuthoringEditPlan, AuthoringEditError> {
    snapshot
        .query(&CancellationToken::new())
        .plan_insert_missing_ids()
}
/// Synchronous edit planning against a completed snapshot.
pub fn plan_insert_missing_id(
    snapshot: &AuthoringSnapshot,
    key: &recite_core::DocumentKey,
    position: SourcePosition,
) -> Result<AuthoringEditPlan, AuthoringEditError> {
    snapshot
        .query(&CancellationToken::new())
        .plan_insert_missing_id(key, position)
}
/// Synchronous edit planning against a completed snapshot.
pub fn plan_insert_missing_ids_for_document(
    snapshot: &AuthoringSnapshot,
    key: &recite_core::DocumentKey,
) -> Result<AuthoringEditPlan, AuthoringEditError> {
    snapshot
        .query(&CancellationToken::new())
        .plan_insert_missing_ids_for_document(key)
}
/// Synchronous edit planning against a completed snapshot.
pub fn plan_insert_missing_ids_in_range(
    snapshot: &AuthoringSnapshot,
    key: &recite_core::DocumentKey,
    range: SourceRange,
) -> Result<AuthoringEditPlan, AuthoringEditError> {
    snapshot
        .query(&CancellationToken::new())
        .plan_insert_missing_ids_in_range(key, range)
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
    pub fn plan_rename_block(
        &self,
        key: &DocumentKey,
        position: SourcePosition,
        new_name: &str,
    ) -> Result<AuthoringEditPlan, AuthoringEditError> {
        plan_rename_block(self, key, position, new_name)
    }
    pub fn plan_insert_missing_ids(&self) -> Result<AuthoringEditPlan, AuthoringEditError> {
        plan_insert_missing_ids(self)
    }
    pub fn plan_insert_missing_id(
        &self,
        key: &recite_core::DocumentKey,
        position: SourcePosition,
    ) -> Result<AuthoringEditPlan, AuthoringEditError> {
        plan_insert_missing_id(self, key, position)
    }
    pub fn plan_insert_missing_ids_for_document(
        &self,
        key: &recite_core::DocumentKey,
    ) -> Result<AuthoringEditPlan, AuthoringEditError> {
        plan_insert_missing_ids_for_document(self, key)
    }
    pub fn plan_insert_missing_ids_in_range(
        &self,
        key: &recite_core::DocumentKey,
        range: SourceRange,
    ) -> Result<AuthoringEditPlan, AuthoringEditError> {
        plan_insert_missing_ids_in_range(self, key, range)
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
