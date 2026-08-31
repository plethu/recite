use recite_core::ProducerIdentity;

use super::freshness::SchemaFreshnessSnapshotIdentity;

/// Which side of a freshness comparison lacked producer identity.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum FreshnessSnapshotSide {
    Expected,
    Actual,
}

/// Failure while constructing host evidence for a schema summary.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum SchemaSummaryEvidenceError {
    #[error("producer identity mismatch: expected {expected:?}, got {actual:?}")]
    ProducerIdentityMismatch {
        expected: ProducerIdentity,
        actual: ProducerIdentity,
    },
    #[error("a producer failure requires supported capability evidence")]
    ContradictoryStates,
    #[error("producer failure code must not be empty")]
    EmptyFailureCode,
    #[error("failed result producer does not match summary producer")]
    FailedResultProducerMismatch {
        expected: ProducerIdentity,
        actual: ProducerIdentity,
    },
    #[error("attached producer result is not failed")]
    FailedResultNotFailed,
    #[error("attached failed result does not carry the current failure")]
    FailedResultFailureMismatch,
    #[error("{side:?} freshness snapshot has no producer identity")]
    MissingSnapshotProducer { side: FreshnessSnapshotSide },
}

/// Failure while applying evidence to a canonical schema.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum SchemaSummaryBuildError {
    #[error("producer identity mismatch: expected {expected:?}, got {actual:?}")]
    ProducerIdentityMismatch {
        expected: ProducerIdentity,
        actual: ProducerIdentity,
    },
    #[error("evidence was supplied for a schema without producer metadata")]
    EvidenceWithoutProducer,
    #[error("freshness expected snapshot identity does not match the summarized schema")]
    FreshnessSchemaMismatch {
        expected: Box<SchemaFreshnessSnapshotIdentity>,
        summarized: Box<SchemaFreshnessSnapshotIdentity>,
    },
    #[error("failed producer result launch snapshot does not match the summarized schema")]
    FailedResultSnapshotMismatch,
}
