use recite_core::{ProducerIdentity, SchemaFingerprint};

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
    #[error("freshness expected fingerprint does not match the summarized schema")]
    FreshnessSchemaMismatch {
        expected: SchemaFingerprint,
        summarized: SchemaFingerprint,
    },
}
