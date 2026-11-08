use recite_core::{BlockId, DocumentKey, StableId};

use super::super::{DocumentVersion, QueryClass, SnapshotGeneration};

/// Structured refusal from compiler-owned edit planning or precondition
/// validation.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum AuthoringEditError {
    #[error(transparent)]
    Interrupted(#[from] crate::authoring::Interrupted),
    #[error("document {document} is not present in the authoring snapshot")]
    UnknownDocument { document: DocumentKey },
    #[error("authoring data for {document} is incomplete for {class:?}")]
    Incomplete {
        document: DocumentKey,
        class: QueryClass,
    },
    #[error("no block symbol was found at {document}:{line}:{column}")]
    NoSymbol {
        document: DocumentKey,
        line: u32,
        column: u32,
    },
    #[error("block symbol at {document}:{line}:{column} is ambiguous")]
    AmbiguousSymbol {
        document: DocumentKey,
        line: u32,
        column: u32,
    },
    #[error("block {block} has more than one definition")]
    AmbiguousBlock { block: BlockId },
    #[error("block name {name:?} is not valid source syntax")]
    InvalidBlockName { name: String },
    #[error("block destination {document}::{block} already exists")]
    DestinationCollision {
        document: DocumentKey,
        block: BlockId,
    },
    #[error("target document {document} is not present")]
    MissingTargetDocument { document: DocumentKey },
    #[error("target document key {document:?} is not valid")]
    InvalidTargetDocument { document: String },
    #[error("block stub target {document}::{block} already exists")]
    TargetAlreadyExists {
        document: DocumentKey,
        block: BlockId,
    },
    #[error("required source span is missing in {document} ({role})")]
    MissingSpan {
        document: DocumentKey,
        role: &'static str,
    },
    #[error("source range cannot be mapped in {document}")]
    UnmappableRange { document: DocumentKey },
    #[error("stable-ID source in {document} is malformed or ambiguous")]
    UnsupportedStableId { document: DocumentKey },
    #[error("stable ID {id:?} is not valid: {reason}")]
    InvalidStableId { id: String, reason: StableIdRefusal },
    #[error("stable ID {id} is already used{}", document.as_ref().map_or_else(String::new, |document| format!(" by catalogue {document}")))]
    StableIdCollision {
        id: StableId,
        /// The catalogue still keying entries on the ID, or `None` when a
        /// line or choice header already carries it.
        document: Option<DocumentKey>,
    },
    #[error("no line or choice carries stable ID {id:?}")]
    UnknownStableId { id: String },
    #[error("stable ID {id} is carried by more than one header; select one by location")]
    AmbiguousStableId { id: StableId },
    #[error("catalogue {document} is not valid PO source")]
    MalformedCatalog { document: DocumentKey },
    #[error("the authoring snapshot does not cover the complete project")]
    IncompleteProject,
    #[error("authoring edit plan has no edits")]
    NoEdits,
    #[error("expected snapshot generation {expected}, but current generation is {actual}")]
    StaleGeneration {
        expected: SnapshotGeneration,
        actual: SnapshotGeneration,
    },
    #[error("planned document {document} is no longer present")]
    StaleDocument { document: DocumentKey },
    #[error("document {document} version changed from {expected:?} to {actual:?}")]
    StaleDocumentVersion {
        document: DocumentKey,
        expected: Option<DocumentVersion>,
        actual: Option<DocumentVersion>,
    },
    #[error("document {document} source text changed")]
    StaleSource { document: DocumentKey },
    #[error("document {document} has duplicate plan preconditions")]
    DuplicatePrecondition { document: DocumentKey },
    #[error("document {document} has an edit without a precondition")]
    MissingPrecondition { document: DocumentKey },
    #[error("document {document} has overlapping edits")]
    OverlappingEdits { document: DocumentKey },
}

/// Why a requested stable ID was refused.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum StableIdRefusal {
    /// The ID does not match the stable-ID grammar.
    Malformed,
    /// The ID contains a character reserved by catalogue contexts or pack
    /// qualifiers: `&`, `:` or `/`.
    ReservedCharacter(char),
}

impl std::fmt::Display for StableIdRefusal {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Malformed => formatter.write_str(
                "use lowercase ASCII letters, digits, `_`, `.` and `-`, starting with a letter or `_`",
            ),
            Self::ReservedCharacter(character) => {
                write!(formatter, "`{character}` is reserved and never appears in an ID")
            }
        }
    }
}
