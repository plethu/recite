use recite_core::{BlockId, DocumentKey, StableId};

use super::range::SourceRange;

/// The compiler-owned reason/provenance for a source edit plan.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum AuthoringEditOperation {
    StableIdInsertion,
    /// The header and every catalogue context keyed on `from` move to `to`.
    RenameStableId {
        from: StableId,
        to: StableId,
    },
    /// One header of a duplicated ID moves to `to`; catalogues stay with the
    /// remaining header.
    ReassignDuplicateStableId {
        from: StableId,
        to: StableId,
    },
    RenameBlock {
        from: BlockId,
        to: BlockId,
    },
    CreateBlockStub {
        source: DocumentKey,
        reference: SourceRange,
        target: DocumentKey,
        block: BlockId,
    },
}
