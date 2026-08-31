use recite_core::{
    ContentFingerprint, ProducerFingerprint, ProducerIdentity, ProjectSchema, SchemaFingerprint,
};

/// Guidance attached to a producer failure for a caller deciding whether to
/// offer a retry action.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ProducerRetryGuidance {
    /// The producer may be retried with the same inputs.
    RetryNow,
    /// A caller should change or repair an external input before retrying.
    RetryAfterCorrection,
    /// Retrying the operation is not expected to help.
    DoNotRetry,
}

impl ProducerRetryGuidance {
    #[must_use]
    pub const fn allows_retry(self) -> bool {
        matches!(self, Self::RetryNow | Self::RetryAfterCorrection)
    }
}

/// Fingerprint evidence exchanged by the host-neutral producer action
/// contract.
///
/// This value contains no schema model or host resource. The schema and
/// content channels describe the currently loaded output, input fingerprints
/// describe the producer precondition, and the output channel identifies the
/// generated producer content. A caller can use a newly loaded schema plus
/// this value to build a fresh [`super::super::SchemaSummary`].
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ProducerActionEvidence {
    producer: ProducerIdentity,
    schema_fingerprint: SchemaFingerprint,
    content_fingerprint: ContentFingerprint,
    input_fingerprints: Vec<ProducerFingerprint>,
    output_fingerprint: Option<ContentFingerprint>,
}

impl ProducerActionEvidence {
    /// Construct fingerprint evidence and normalise producer inputs into their
    /// deterministic order.
    #[must_use]
    pub fn new(
        producer: ProducerIdentity,
        schema_fingerprint: SchemaFingerprint,
        content_fingerprint: ContentFingerprint,
        input_fingerprints: impl IntoIterator<Item = ProducerFingerprint>,
        output_fingerprint: Option<ContentFingerprint>,
    ) -> Self {
        let mut input_fingerprints = input_fingerprints.into_iter().collect::<Vec<_>>();
        input_fingerprints.sort();
        Self {
            producer,
            schema_fingerprint,
            content_fingerprint,
            input_fingerprints,
            output_fingerprint,
        }
    }

    /// Extract the producer action evidence represented by a canonical schema.
    pub fn from_schema(schema: &ProjectSchema) -> Result<Self, ProducerActionEvidenceError> {
        let metadata = schema
            .producer_metadata
            .as_ref()
            .ok_or(ProducerActionEvidenceError::MissingProducerMetadata)?;
        let producer = metadata
            .producer
            .clone()
            .ok_or(ProducerActionEvidenceError::MissingProducerIdentity)?;
        Ok(Self::new(
            producer,
            schema.canonical_fingerprint(),
            schema.canonical_content_fingerprint(),
            metadata.producer_fingerprints.clone(),
            metadata.content_fingerprint.clone(),
        ))
    }

    #[must_use]
    pub const fn producer(&self) -> &ProducerIdentity {
        &self.producer
    }

    #[must_use]
    pub const fn schema_fingerprint(&self) -> &SchemaFingerprint {
        &self.schema_fingerprint
    }

    #[must_use]
    pub const fn content_fingerprint(&self) -> &ContentFingerprint {
        &self.content_fingerprint
    }

    #[must_use]
    pub fn input_fingerprints(&self) -> &[ProducerFingerprint] {
        &self.input_fingerprints
    }

    #[must_use]
    pub const fn output_fingerprint(&self) -> Option<&ContentFingerprint> {
        self.output_fingerprint.as_ref()
    }
}

/// Why canonical producer action evidence could not be extracted.
#[derive(Clone, Copy, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ProducerActionEvidenceError {
    #[error("schema has no producer metadata")]
    MissingProducerMetadata,
    #[error("producer metadata has no producer identity")]
    MissingProducerIdentity,
}
