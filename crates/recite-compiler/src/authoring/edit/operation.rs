use recite_core::{BlockId, DocumentKey};

use super::range::SourceRange;

/// The compiler-owned reason/provenance for a source edit plan.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum AuthoringEditOperation {
    StableIdInsertion,
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
