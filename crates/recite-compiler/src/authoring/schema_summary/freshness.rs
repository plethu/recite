use recite_core::{
    ProducerIdentity, ProjectSchema, SchemaFingerprint, SchemaProducerFreshness,
    compare_schema_producer_freshness_detailed,
};

use super::errors::{FreshnessSnapshotSide, SchemaSummaryEvidenceError};

/// Producer freshness comparison bound to both canonical snapshots.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct SchemaFreshnessEvidence {
    expected_producer: ProducerIdentity,
    actual_producer: ProducerIdentity,
    expected_schema_fingerprint: SchemaFingerprint,
    actual_schema_fingerprint: SchemaFingerprint,
    comparison: SchemaProducerFreshness,
}

impl SchemaFreshnessEvidence {
    /// Compare two canonical snapshots after requiring matching producer
    /// identities on both sides.
    pub fn from_snapshots(
        expected: &ProjectSchema,
        actual: &ProjectSchema,
    ) -> Result<Self, SchemaSummaryEvidenceError> {
        let expected_producer = producer(expected, FreshnessSnapshotSide::Expected)?;
        let actual_producer = producer(actual, FreshnessSnapshotSide::Actual)?;
        if expected_producer != actual_producer {
            return Err(SchemaSummaryEvidenceError::ProducerIdentityMismatch {
                expected: expected_producer.clone(),
                actual: actual_producer.clone(),
            });
        }
        Ok(Self {
            expected_producer: expected_producer.clone(),
            actual_producer: actual_producer.clone(),
            expected_schema_fingerprint: expected.canonical_fingerprint(),
            actual_schema_fingerprint: actual.canonical_fingerprint(),
            comparison: compare_schema_producer_freshness_detailed(expected, actual),
        })
    }

    #[must_use]
    pub const fn expected_producer(&self) -> &ProducerIdentity {
        &self.expected_producer
    }

    #[must_use]
    pub const fn actual_producer(&self) -> &ProducerIdentity {
        &self.actual_producer
    }

    #[must_use]
    pub const fn expected_schema_fingerprint(&self) -> &SchemaFingerprint {
        &self.expected_schema_fingerprint
    }

    #[must_use]
    pub const fn actual_schema_fingerprint(&self) -> &SchemaFingerprint {
        &self.actual_schema_fingerprint
    }

    #[must_use]
    pub const fn comparison(&self) -> &SchemaProducerFreshness {
        &self.comparison
    }
}

fn producer(
    schema: &ProjectSchema,
    side: FreshnessSnapshotSide,
) -> Result<&ProducerIdentity, SchemaSummaryEvidenceError> {
    schema
        .producer_metadata
        .as_ref()
        .and_then(|metadata| metadata.producer.as_ref())
        .ok_or(SchemaSummaryEvidenceError::MissingSnapshotProducer { side })
}
