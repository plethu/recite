use std::collections::BTreeMap;

use recite_core::{
    ContentFingerprint, MetadataDomainDefinition, ProducerFingerprint, ProducerIdentity,
    ProjectSchema, SchemaFingerprint, SchemaProducerFreshness,
    compare_schema_producer_freshness_detailed,
};

use super::errors::{FreshnessSnapshotSide, SchemaSummaryEvidenceError};

/// Normalized identity of one producer freshness snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct SchemaFreshnessSnapshotIdentity {
    semantic_fingerprint: SchemaFingerprint,
    producer: ProducerIdentity,
    content_fingerprint: Option<ContentFingerprint>,
    manifest_producer_fingerprints: Vec<ProducerFingerprint>,
    registry_producer_fingerprints: BTreeMap<String, Vec<ProducerFingerprint>>,
    metadata_domain_producer_fingerprints: BTreeMap<String, Vec<ProducerFingerprint>>,
}

impl SchemaFreshnessSnapshotIdentity {
    pub(crate) fn from_schema(
        schema: &ProjectSchema,
        side: FreshnessSnapshotSide,
    ) -> Result<Self, SchemaSummaryEvidenceError> {
        let producer_metadata = schema.producer_metadata.as_ref();
        let producer = producer_metadata
            .and_then(|metadata| metadata.producer.as_ref())
            .ok_or(SchemaSummaryEvidenceError::MissingSnapshotProducer { side })?;
        Ok(Self {
            semantic_fingerprint: schema.canonical_fingerprint(),
            producer: producer.clone(),
            content_fingerprint: producer_metadata
                .and_then(|metadata| metadata.content_fingerprint.clone()),
            manifest_producer_fingerprints: producer_metadata.map_or_else(Vec::new, |metadata| {
                sorted_fingerprints(&metadata.producer_fingerprints)
            }),
            registry_producer_fingerprints: schema
                .registries
                .iter()
                .map(|(name, definition)| {
                    (
                        name.clone(),
                        sorted_fingerprints(&definition.producer_fingerprints),
                    )
                })
                .collect(),
            metadata_domain_producer_fingerprints: schema
                .metadata_domains
                .iter()
                .map(|(name, definition)| {
                    (
                        name.clone(),
                        sorted_fingerprints(domain_fingerprints(definition)),
                    )
                })
                .collect(),
        })
    }

    #[must_use]
    pub const fn semantic_fingerprint(&self) -> &SchemaFingerprint {
        &self.semantic_fingerprint
    }

    #[must_use]
    pub const fn producer(&self) -> &ProducerIdentity {
        &self.producer
    }

    #[must_use]
    pub const fn content_fingerprint(&self) -> Option<&ContentFingerprint> {
        self.content_fingerprint.as_ref()
    }

    #[must_use]
    pub fn manifest_producer_fingerprints(&self) -> &[ProducerFingerprint] {
        &self.manifest_producer_fingerprints
    }

    #[must_use]
    pub fn registry_producer_fingerprints(&self) -> &BTreeMap<String, Vec<ProducerFingerprint>> {
        &self.registry_producer_fingerprints
    }

    #[must_use]
    pub fn metadata_domain_producer_fingerprints(
        &self,
    ) -> &BTreeMap<String, Vec<ProducerFingerprint>> {
        &self.metadata_domain_producer_fingerprints
    }
}

/// Producer freshness comparison bound to both canonical snapshots.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct SchemaFreshnessEvidence {
    expected_identity: SchemaFreshnessSnapshotIdentity,
    actual_identity: SchemaFreshnessSnapshotIdentity,
    comparison: SchemaProducerFreshness,
}

impl SchemaFreshnessEvidence {
    /// Compare two canonical snapshots after requiring matching producer
    /// identities on both sides.
    pub fn from_snapshots(
        expected: &ProjectSchema,
        actual: &ProjectSchema,
    ) -> Result<Self, SchemaSummaryEvidenceError> {
        let expected_identity = SchemaFreshnessSnapshotIdentity::from_schema(
            expected,
            FreshnessSnapshotSide::Expected,
        )?;
        let actual_identity =
            SchemaFreshnessSnapshotIdentity::from_schema(actual, FreshnessSnapshotSide::Actual)?;
        if expected_identity.producer() != actual_identity.producer() {
            return Err(SchemaSummaryEvidenceError::ProducerIdentityMismatch {
                expected: expected_identity.producer().clone(),
                actual: actual_identity.producer().clone(),
            });
        }
        Ok(Self {
            expected_identity,
            actual_identity,
            comparison: compare_schema_producer_freshness_detailed(expected, actual),
        })
    }

    #[must_use]
    pub const fn expected_identity(&self) -> &SchemaFreshnessSnapshotIdentity {
        &self.expected_identity
    }

    #[must_use]
    pub const fn actual_identity(&self) -> &SchemaFreshnessSnapshotIdentity {
        &self.actual_identity
    }

    #[must_use]
    pub const fn expected_producer(&self) -> &ProducerIdentity {
        self.expected_identity.producer()
    }

    #[must_use]
    pub const fn actual_producer(&self) -> &ProducerIdentity {
        self.actual_identity.producer()
    }

    #[must_use]
    pub const fn comparison(&self) -> &SchemaProducerFreshness {
        &self.comparison
    }
}

fn sorted_fingerprints(fingerprints: &[ProducerFingerprint]) -> Vec<ProducerFingerprint> {
    let mut sorted = fingerprints.to_vec();
    sorted.sort();
    sorted
}

fn domain_fingerprints(domain: &MetadataDomainDefinition) -> &[ProducerFingerprint] {
    match domain {
        MetadataDomainDefinition::Flat(domain) => &domain.provenance.producer_fingerprints,
        MetadataDomainDefinition::Contextual(domain) => &domain.provenance.producer_fingerprints,
    }
}
