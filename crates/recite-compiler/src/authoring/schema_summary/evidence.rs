use recite_core::{
    ProducerIdentity, ProjectSchema, SchemaProducerFreshness,
    compare_schema_producer_freshness_detailed,
};

/// Producer capability evidence supplied by a host or producer report.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProducerCapabilityStatus {
    Supported,
    ReadOnly,
    Unavailable,
}

/// A structured current producer failure. It is evidence only; the compiler
/// never interprets `code` as a command or executes a retry.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ProducerFailureEvidence {
    producer: ProducerIdentity,
    code: String,
    detail: Option<String>,
}

impl ProducerFailureEvidence {
    /// Creates a structured failure associated with one producer identity.
    pub fn new(
        producer: ProducerIdentity,
        code: impl Into<String>,
        detail: Option<String>,
    ) -> Result<Self, SchemaSummaryEvidenceError> {
        let code = code.into();
        if code.trim().is_empty() {
            return Err(SchemaSummaryEvidenceError::EmptyFailureCode);
        }
        Ok(Self {
            producer,
            code,
            detail,
        })
    }

    #[must_use]
    pub const fn producer(&self) -> &ProducerIdentity {
        &self.producer
    }

    #[must_use]
    pub fn code(&self) -> &str {
        &self.code
    }

    #[must_use]
    pub fn detail(&self) -> Option<&str> {
        self.detail.as_deref()
    }
}

/// Typed evidence used to enrich a generated schema summary.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct SchemaSummaryEvidence {
    pub(super) producer: ProducerIdentity,
    pub(super) capability: Option<ProducerCapabilityStatus>,
    pub(super) current_failure: Option<ProducerFailureEvidence>,
    pub(super) freshness: Option<SchemaProducerFreshness>,
}

impl SchemaSummaryEvidence {
    #[must_use]
    pub fn builder(producer: ProducerIdentity) -> SchemaSummaryEvidenceBuilder {
        SchemaSummaryEvidenceBuilder {
            producer,
            capability: None,
            current_failure: None,
            freshness: None,
        }
    }

    #[must_use]
    pub const fn producer(&self) -> &ProducerIdentity {
        &self.producer
    }

    #[must_use]
    pub const fn capability(&self) -> Option<ProducerCapabilityStatus> {
        self.capability
    }

    #[must_use]
    pub const fn current_failure(&self) -> Option<&ProducerFailureEvidence> {
        self.current_failure.as_ref()
    }

    #[must_use]
    pub const fn freshness(&self) -> Option<&SchemaProducerFreshness> {
        self.freshness.as_ref()
    }
}

/// Builder for one identity-checked, deterministic evidence input.
#[derive(Clone, Debug)]
pub struct SchemaSummaryEvidenceBuilder {
    producer: ProducerIdentity,
    capability: Option<ProducerCapabilityStatus>,
    current_failure: Option<ProducerFailureEvidence>,
    freshness: Option<SchemaProducerFreshness>,
}

impl SchemaSummaryEvidenceBuilder {
    #[must_use]
    pub fn capability(mut self, capability: ProducerCapabilityStatus) -> Self {
        self.capability = Some(capability);
        self
    }

    #[must_use]
    pub fn current_failure(mut self, failure: ProducerFailureEvidence) -> Self {
        self.current_failure = Some(failure);
        self
    }

    /// Compare the supplied canonical snapshots while retaining every core
    /// freshness channel (content, manifest, registries, and domains).
    pub fn compare_freshness(
        mut self,
        expected: &ProjectSchema,
        actual: &ProjectSchema,
    ) -> Result<Self, SchemaSummaryEvidenceError> {
        for schema in [expected, actual] {
            if let Some(identity) = schema
                .producer_metadata
                .as_ref()
                .and_then(|metadata| metadata.producer.as_ref())
                && identity != &self.producer
            {
                return Err(SchemaSummaryEvidenceError::ProducerIdentityMismatch {
                    expected: self.producer.clone(),
                    actual: identity.clone(),
                });
            }
        }
        self.freshness = Some(compare_schema_producer_freshness_detailed(expected, actual));
        Ok(self)
    }

    #[must_use]
    pub fn freshness(mut self, freshness: SchemaProducerFreshness) -> Self {
        self.freshness = Some(freshness);
        self
    }

    /// Validate producer identity and capability/failure consistency.
    pub fn build(self) -> Result<SchemaSummaryEvidence, SchemaSummaryEvidenceError> {
        if let Some(failure) = &self.current_failure {
            if failure.producer != self.producer {
                return Err(SchemaSummaryEvidenceError::ProducerIdentityMismatch {
                    expected: self.producer,
                    actual: failure.producer.clone(),
                });
            }
            if self.capability != Some(ProducerCapabilityStatus::Supported) {
                return Err(SchemaSummaryEvidenceError::ContradictoryStates);
            }
        }
        Ok(SchemaSummaryEvidence {
            producer: self.producer,
            capability: self.capability,
            current_failure: self.current_failure,
            freshness: self.freshness,
        })
    }
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
}
