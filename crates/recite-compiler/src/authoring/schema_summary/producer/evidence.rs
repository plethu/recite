use recite_core::{ContentFingerprint, ProducerIdentity, ProjectSchema, SchemaFingerprint};

use super::scopes::{
    ProducerFingerprintScopes, ProducerFingerprintScopesError, ProducerLaunchSnapshot,
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

/// Canonical evidence from a reloaded producer output.
///
/// This value can only be constructed from a canonical [`ProjectSchema`].
/// It contains no schema model, host resource, URI, or execution handle.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ProducerActionOutputEvidence {
    producer: ProducerIdentity,
    schema_fingerprint: SchemaFingerprint,
    content_fingerprint: ContentFingerprint,
    input_fingerprints: ProducerFingerprintScopes,
    output_fingerprint: Option<ContentFingerprint>,
}

impl ProducerActionOutputEvidence {
    /// Extract canonical output evidence from a reloaded project schema.
    pub fn from_schema(schema: &ProjectSchema) -> Result<Self, ProducerActionEvidenceError> {
        let metadata = schema
            .producer_metadata
            .as_ref()
            .ok_or(ProducerActionEvidenceError::MissingProducerMetadata)?;
        let producer = metadata
            .producer
            .clone()
            .ok_or(ProducerActionEvidenceError::MissingProducerIdentity)?;
        let input_fingerprints = ProducerFingerprintScopes::from_schema(schema)
            .map_err(ProducerActionEvidenceError::InvalidInputFingerprints)?;
        Ok(Self {
            producer,
            schema_fingerprint: schema.canonical_fingerprint(),
            content_fingerprint: schema.canonical_content_fingerprint(),
            input_fingerprints,
            output_fingerprint: metadata.content_fingerprint.clone(),
        })
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
    pub const fn input_fingerprints(&self) -> &ProducerFingerprintScopes {
        &self.input_fingerprints
    }

    #[must_use]
    pub const fn output_fingerprint(&self) -> Option<&ContentFingerprint> {
        self.output_fingerprint.as_ref()
    }

    #[must_use]
    pub fn launch_snapshot(&self) -> ProducerLaunchSnapshot {
        ProducerLaunchSnapshot::new(self.producer.clone(), self.input_fingerprints.clone())
    }
}

/// Compatibility-friendly name for the canonical producer output evidence.
pub type ProducerActionEvidence = ProducerActionOutputEvidence;

/// Why canonical producer action evidence could not be extracted.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ProducerActionEvidenceError {
    #[error("schema has no producer metadata")]
    MissingProducerMetadata,
    #[error("producer metadata has no producer identity")]
    MissingProducerIdentity,
    #[error("producer output input fingerprints are invalid: {0}")]
    InvalidInputFingerprints(#[source] ProducerFingerprintScopesError),
}
