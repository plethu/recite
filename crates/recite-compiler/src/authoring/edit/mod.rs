mod error;
mod fingerprint;
mod helpers;
mod operation;
mod plan;
mod plan_rename;
mod plan_stable_id_rename;
mod plan_stable_ids;
mod plan_stable_ids_scope;
mod plan_stub;
mod precondition;
mod range;
mod stable_id_target;
mod stable_selection;
mod types;
mod validate;

mod synchronous;
pub use stable_id_target::{StableIdOccurrence, StableIdTarget};
pub use synchronous::{
    plan_create_block_stub, plan_create_block_stub_in_range, plan_insert_missing_id,
    plan_insert_missing_ids, plan_insert_missing_ids_for_document,
    plan_insert_missing_ids_in_range, plan_reassign_stable_id, plan_rename_block,
    plan_rename_stable_id,
};
pub use types::{
    AuthoringEditError, AuthoringEditOperation, AuthoringEditPlan, EditPrecondition, SourceEdit,
    SourceFingerprint, SourceRange, StableIdRefusal,
};
