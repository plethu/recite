use recite_core::{ProducerIdentity, ProjectSchema};

use super::errors::SchemaSummaryEvidenceError;
use super::freshness::SchemaFreshnessEvidence;
use super::producer::{ProducerActionResult, ProducerActionStatus, ProducerRetryGuidance};

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
    retry_guidance: ProducerRetryGuidance,
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
            retry_guidance: ProducerRetryGuidance::RetryNow,
        })
    }

    /// Attach typed guidance for callers deciding whether to offer a retry.
    #[must_use]
    pub const fn with_retry_guidance(mut self, guidance: ProducerRetryGuidance) -> Self {
        self.retry_guidance = guidance;
        self
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

    #[must_use]
    pub const fn retry_guidance(&self) -> ProducerRetryGuidance {
        self.retry_guidance
    }
}

/// Typed evidence used to enrich a generated schema summary.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct SchemaSummaryEvidence {
    pub(super) producer: ProducerIdentity,
    pub(super) capability: Option<ProducerCapabilityStatus>,
    pub(super) current_failure: Option<ProducerFailureEvidence>,
    pub(super) failed_result: Option<ProducerActionResult>,
    pub(super) freshness: Option<SchemaFreshnessEvidence>,
}

impl SchemaSummaryEvidence {
    #[must_use]
    pub fn builder(producer: ProducerIdentity) -> SchemaSummaryEvidenceBuilder {
        SchemaSummaryEvidenceBuilder {
            producer,
            capability: None,
            current_failure: None,
            failed_result: None,
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
    pub const fn failed_result(&self) -> Option<&ProducerActionResult> {
        self.failed_result.as_ref()
    }

    #[must_use]
    pub const fn freshness(&self) -> Option<&SchemaFreshnessEvidence> {
        self.freshness.as_ref()
    }
}

/// Builder for one identity-checked, deterministic evidence input.
#[derive(Clone, Debug)]
pub struct SchemaSummaryEvidenceBuilder {
    producer: ProducerIdentity,
    capability: Option<ProducerCapabilityStatus>,
    current_failure: Option<ProducerFailureEvidence>,
    failed_result: Option<ProducerActionResult>,
    freshness: Option<SchemaFreshnessEvidence>,
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

    /// Attach the exact failed result from which a retry descriptor may be
    /// projected. A bare current failure remains insufficient for that.
    #[must_use]
    pub fn failed_result(mut self, result: ProducerActionResult) -> Self {
        self.failed_result = Some(result);
        self
    }

    /// Compare the supplied canonical snapshots while retaining every core
    /// freshness channel (content, manifest, registries, and domains).
    pub fn compare_freshness(
        mut self,
        expected: &ProjectSchema,
        actual: &ProjectSchema,
    ) -> Result<Self, SchemaSummaryEvidenceError> {
        let freshness = SchemaFreshnessEvidence::from_snapshots(expected, actual)?;
        if freshness.expected_producer() != &self.producer {
            return Err(SchemaSummaryEvidenceError::ProducerIdentityMismatch {
                expected: self.producer.clone(),
                actual: freshness.expected_producer().clone(),
            });
        }
        self.freshness = Some(freshness);
        Ok(self)
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
        if let Some(result) = &self.failed_result {
            if result.request().producer() != &self.producer {
                return Err(SchemaSummaryEvidenceError::FailedResultProducerMismatch {
                    expected: self.producer,
                    actual: result.request().producer().clone(),
                });
            }
            if result.status() != ProducerActionStatus::Failed {
                return Err(SchemaSummaryEvidenceError::FailedResultNotFailed);
            }
            if let Some(current_failure) = &self.current_failure
                && result.failure() != Some(current_failure)
            {
                return Err(SchemaSummaryEvidenceError::FailedResultFailureMismatch);
            }
        }
        Ok(SchemaSummaryEvidence {
            producer: self.producer,
            capability: self.capability,
            current_failure: self.current_failure,
            failed_result: self.failed_result,
            freshness: self.freshness,
        })
    }
}
