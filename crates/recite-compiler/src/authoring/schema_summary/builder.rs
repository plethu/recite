use recite_core::{ContentFingerprint, ProjectSchema, SchemaSource};

use super::dialogue::{RegistrySummary, SchemaTypeSummary, SpeakerSummary};
use super::evidence::SchemaSummaryEvidence;
use super::freshness::{SchemaFreshnessEvidence, SchemaFreshnessSnapshotIdentity};
use super::functions::{AvailabilityReasonSummary, ConditionSummary, EffectSummary, MarkupSummary};
use super::helpers::{
    capability, domain_origin, freshness, ownership, provenance, sorted_fingerprints,
};
use super::metadata::{MetadataDomainSummary, MetadataKeySummary};
use super::projections::{PresentationProjectorSummary, ProjectionQueryFunctionSummary};
use super::{
    ProducerMetadataSummary, SchemaFingerprintSummary, SchemaSourceSummary, SchemaSummary,
};
use super::{SchemaFreshness, SchemaSummaryBuildError};

impl SchemaSummary {
    /// Build a summary directly from the canonical semantic schema.
    #[must_use]
    pub fn from_schema(schema: &ProjectSchema) -> Self {
        Self::build(schema, None, false, None)
    }

    /// Build a summary from a source-owning schema while retaining its exact
    /// source-owned fingerprint. The canonical schema remains the semantic
    /// authority.
    #[must_use]
    pub fn from_source(source: &SchemaSource) -> Self {
        Self::build(
            source.schema(),
            Some(source.source_fingerprint().clone()),
            true,
            None,
        )
    }

    /// Build a generated/read-only summary with explicit host evidence.
    pub fn from_schema_with_evidence(
        schema: &ProjectSchema,
        evidence: Option<&SchemaSummaryEvidence>,
    ) -> Result<Self, SchemaSummaryBuildError> {
        validate_evidence(schema, evidence)?;
        Ok(Self::build(schema, None, false, evidence))
    }

    fn build(
        schema: &ProjectSchema,
        source_owned_fingerprint: Option<ContentFingerprint>,
        source_owned: bool,
        evidence: Option<&SchemaSummaryEvidence>,
    ) -> Self {
        let owner = ownership(schema, source_owned);
        let canonical_content = schema.canonical_content_fingerprint();
        let producer = schema.producer_metadata.as_ref();
        let freshness = evidence
            .and_then(SchemaSummaryEvidence::freshness)
            .map(SchemaFreshnessEvidence::comparison)
            .cloned()
            .map(|freshness| SchemaFreshness::Compared(Box::new(freshness)))
            .unwrap_or_else(|| freshness(producer.is_some()));
        let producer_inputs = producer.map_or_else(Vec::new, |metadata| {
            sorted_fingerprints(&metadata.producer_fingerprints)
        });
        let fingerprints = SchemaFingerprintSummary {
            semantic: schema.canonical_fingerprint(),
            canonical_content,
            source_owned: source_owned_fingerprint,
            producer_content: producer.and_then(|metadata| metadata.content_fingerprint.clone()),
            producer_inputs,
        };
        let producer_metadata = producer.map(|metadata| ProducerMetadataSummary {
            producer: metadata.producer.clone(),
            content_fingerprint: metadata.content_fingerprint.clone(),
            schema_export_version: metadata.schema_export_version,
            inclusion_policy: metadata.inclusion_policy.clone(),
            producer_fingerprints: sorted_fingerprints(&metadata.producer_fingerprints),
            freshness: freshness.clone(),
        });

        Self {
            schema_version: schema.schema_version,
            source: SchemaSourceSummary {
                ownership: owner.clone(),
            },
            capability: capability(&owner, false, source_owned, evidence),
            fingerprints,
            producer_metadata,
            freshness,
            types: schema
                .types
                .iter()
                .map(|(name, definition)| SchemaTypeSummary {
                    name: name.clone(),
                    definition: definition.clone(),
                    provenance: provenance(&owner, None),
                    capability: capability(&owner, false, source_owned, evidence),
                })
                .collect(),
            registries: schema
                .registries
                .iter()
                .map(|(name, definition)| RegistrySummary {
                    name: name.clone(),
                    definition: definition.clone(),
                    provenance: provenance(&owner, definition.origin.as_ref()),
                    capability: capability(
                        &owner,
                        definition.origin.is_some(),
                        source_owned,
                        evidence,
                    ),
                })
                .collect(),
            speakers: schema
                .speakers
                .iter()
                .map(|(name, definition)| SpeakerSummary {
                    name: name.clone(),
                    definition: definition.clone(),
                    provenance: provenance(&owner, None),
                    capability: capability(&owner, false, source_owned, evidence),
                })
                .collect(),
            conditions: schema
                .conditions
                .iter()
                .map(|(name, definition)| ConditionSummary {
                    name: name.clone(),
                    definition: definition.clone(),
                    provenance: provenance(&owner, None),
                    capability: capability(&owner, false, source_owned, evidence),
                })
                .collect(),
            availability_reasons: schema
                .availability_reasons
                .iter()
                .map(|(id, definition)| AvailabilityReasonSummary {
                    id: id.clone(),
                    definition: definition.clone(),
                    provenance: provenance(&owner, definition.origin.as_ref()),
                    capability: capability(
                        &owner,
                        definition.origin.is_some(),
                        source_owned,
                        evidence,
                    ),
                })
                .collect(),
            effects: schema
                .effects
                .iter()
                .map(|(name, definition)| EffectSummary {
                    name: name.clone(),
                    definition: definition.clone(),
                    provenance: provenance(&owner, None),
                    capability: capability(&owner, false, source_owned, evidence),
                })
                .collect(),
            metadata_domains: schema
                .metadata_domains
                .iter()
                .map(|(name, definition)| MetadataDomainSummary {
                    name: name.clone(),
                    definition: definition.clone(),
                    provenance: provenance(&owner, domain_origin(definition)),
                    capability: capability(
                        &owner,
                        domain_origin(definition).is_some(),
                        source_owned,
                        evidence,
                    ),
                })
                .collect(),
            metadata: schema
                .metadata
                .iter()
                .map(|(name, definition)| MetadataKeySummary {
                    name: name.clone(),
                    definition: definition.clone(),
                    provenance: provenance(&owner, None),
                    capability: capability(&owner, false, source_owned, evidence),
                })
                .collect(),
            projection_queries: schema
                .projection_queries
                .iter()
                .map(|(name, definition)| ProjectionQueryFunctionSummary {
                    name: name.clone(),
                    definition: definition.clone(),
                    provenance: provenance(&owner, None),
                    capability: capability(&owner, false, source_owned, evidence),
                })
                .collect(),
            presentation_projectors: schema
                .presentation_projectors
                .iter()
                .map(|(name, definition)| PresentationProjectorSummary {
                    name: name.clone(),
                    definition: definition.clone(),
                    provenance: provenance(&owner, None),
                    capability: capability(&owner, false, source_owned, evidence),
                })
                .collect(),
            markup: schema
                .markup
                .iter()
                .map(|(name, definition)| MarkupSummary {
                    name: name.clone(),
                    definition: definition.clone(),
                    provenance: provenance(&owner, None),
                    capability: capability(&owner, false, source_owned, evidence),
                })
                .collect(),
        }
    }
}

fn validate_evidence(
    schema: &ProjectSchema,
    evidence: Option<&SchemaSummaryEvidence>,
) -> Result<(), SchemaSummaryBuildError> {
    if let Some(evidence) = evidence {
        let expected = schema
            .producer_metadata
            .as_ref()
            .and_then(|metadata| metadata.producer.as_ref())
            .ok_or(SchemaSummaryBuildError::EvidenceWithoutProducer)?;
        if expected != evidence.producer() {
            return Err(SchemaSummaryBuildError::ProducerIdentityMismatch {
                expected: expected.clone(),
                actual: evidence.producer().clone(),
            });
        }
        if let Some(freshness) = evidence.freshness() {
            let summarized = SchemaFreshnessSnapshotIdentity::from_schema(
                schema,
                super::errors::FreshnessSnapshotSide::Expected,
            )
            .map_err(|_| SchemaSummaryBuildError::EvidenceWithoutProducer)?;
            if freshness.expected_identity() != &summarized {
                return Err(SchemaSummaryBuildError::FreshnessSchemaMismatch {
                    expected: Box::new(freshness.expected_identity().clone()),
                    summarized: Box::new(summarized),
                });
            }
        }
    }
    Ok(())
}
