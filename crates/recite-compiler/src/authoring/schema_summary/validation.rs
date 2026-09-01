use recite_core::ProjectSchema;

use super::errors::FreshnessSnapshotSide;
use super::errors::SchemaSummaryBuildError;
use super::evidence::SchemaSummaryEvidence;
use super::freshness::SchemaFreshnessSnapshotIdentity;

pub(super) fn validate_evidence(
    schema: &ProjectSchema,
    evidence: Option<&SchemaSummaryEvidence>,
) -> Result<(), SchemaSummaryBuildError> {
    let Some(evidence) = evidence else {
        return Ok(());
    };
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
        let summarized =
            SchemaFreshnessSnapshotIdentity::from_schema(schema, FreshnessSnapshotSide::Expected)
                .map_err(|_| SchemaSummaryBuildError::EvidenceWithoutProducer)?;
        if freshness.expected_identity() != &summarized {
            return Err(SchemaSummaryBuildError::FreshnessSchemaMismatch {
                expected: Box::new(freshness.expected_identity().clone()),
                summarized: Box::new(summarized),
            });
        }
    }
    Ok(())
}
