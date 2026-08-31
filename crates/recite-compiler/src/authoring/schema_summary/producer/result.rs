use recite_core::ProducerIdentity;

use super::super::evidence::ProducerFailureEvidence;
use super::evidence::ProducerActionEvidence;
use super::request::ProducerActionRequest;

/// Terminal state of one producer action result.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub enum ProducerActionStatus {
    Succeeded,
    Failed,
    Cancelled,
    Stale,
}

/// Exclusive payload of a producer action result.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProducerActionResultOutcome {
    Succeeded { evidence: ProducerActionEvidence },
    Failed { failure: ProducerFailureEvidence },
    Cancelled,
    Stale { observed: ProducerActionEvidence },
}

/// A producer result bound to the exact request that created it.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ProducerActionResult {
    request_identity: super::request::ProducerActionRequestIdentity,
    outcome: ProducerActionResultOutcome,
}

impl ProducerActionResult {
    /// Record successful producer output evidence. The input fingerprints must
    /// still match the request precondition; schema and output fingerprints may
    /// legitimately change after regeneration.
    pub fn succeeded(
        request: &ProducerActionRequest,
        evidence: ProducerActionEvidence,
    ) -> Result<Self, ProducerActionResultError> {
        validate_output_evidence(request, &evidence)?;
        Ok(Self {
            request_identity: request.identity().clone(),
            outcome: ProducerActionResultOutcome::Succeeded { evidence },
        })
    }

    /// Record a structured producer failure.
    pub fn failed(
        request: &ProducerActionRequest,
        failure: ProducerFailureEvidence,
    ) -> Result<Self, ProducerActionResultError> {
        if failure.producer() != request.producer() {
            return Err(ProducerActionResultError::ProducerIdentityMismatch {
                expected: request.producer().clone(),
                actual: failure.producer().clone(),
            });
        }
        Ok(Self {
            request_identity: request.identity().clone(),
            outcome: ProducerActionResultOutcome::Failed { failure },
        })
    }

    /// Record a host cancellation without inventing a producer failure.
    #[must_use]
    pub fn cancelled(request: &ProducerActionRequest) -> Self {
        Self {
            request_identity: request.identity().clone(),
            outcome: ProducerActionResultOutcome::Cancelled,
        }
    }

    /// Record a refusal because the caller observed a changed precondition.
    pub fn stale(
        request: &ProducerActionRequest,
        observed: ProducerActionEvidence,
    ) -> Result<Self, ProducerActionResultError> {
        validate_observed_evidence(request, &observed)?;
        if observed == *request.expected() {
            return Err(ProducerActionResultError::StaleWithoutChangedEvidence);
        }
        Ok(Self {
            request_identity: request.identity().clone(),
            outcome: ProducerActionResultOutcome::Stale { observed },
        })
    }

    #[must_use]
    pub const fn request_identity(&self) -> &super::request::ProducerActionRequestIdentity {
        &self.request_identity
    }

    #[must_use]
    pub const fn status(&self) -> ProducerActionStatus {
        match self.outcome {
            ProducerActionResultOutcome::Succeeded { .. } => ProducerActionStatus::Succeeded,
            ProducerActionResultOutcome::Failed { .. } => ProducerActionStatus::Failed,
            ProducerActionResultOutcome::Cancelled => ProducerActionStatus::Cancelled,
            ProducerActionResultOutcome::Stale { .. } => ProducerActionStatus::Stale,
        }
    }

    #[must_use]
    pub const fn outcome(&self) -> &ProducerActionResultOutcome {
        &self.outcome
    }

    #[must_use]
    pub const fn evidence(&self) -> Option<&ProducerActionEvidence> {
        match &self.outcome {
            ProducerActionResultOutcome::Succeeded { evidence }
            | ProducerActionResultOutcome::Stale { observed: evidence } => Some(evidence),
            ProducerActionResultOutcome::Failed { .. } | ProducerActionResultOutcome::Cancelled => {
                None
            }
        }
    }

    #[must_use]
    pub const fn failure(&self) -> Option<&ProducerFailureEvidence> {
        match &self.outcome {
            ProducerActionResultOutcome::Failed { failure } => Some(failure),
            ProducerActionResultOutcome::Succeeded { .. }
            | ProducerActionResultOutcome::Cancelled
            | ProducerActionResultOutcome::Stale { .. } => None,
        }
    }

    #[must_use]
    pub const fn observed_stale_evidence(&self) -> Option<&ProducerActionEvidence> {
        match &self.outcome {
            ProducerActionResultOutcome::Stale { observed } => Some(observed),
            ProducerActionResultOutcome::Succeeded { .. }
            | ProducerActionResultOutcome::Failed { .. }
            | ProducerActionResultOutcome::Cancelled => None,
        }
    }

    /// Verify that this result belongs to the supplied request.
    pub fn validate_for(
        &self,
        request: &ProducerActionRequest,
    ) -> Result<(), ProducerActionResultError> {
        if self.request_identity != *request.identity() {
            return Err(ProducerActionResultError::RequestIdentityMismatch);
        }
        Ok(())
    }
}

/// Failure while constructing or validating a producer action result.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ProducerActionResultError {
    #[error("producer result request identity does not match the supplied request")]
    RequestIdentityMismatch,
    #[error("producer result evidence belongs to a different producer")]
    ProducerIdentityMismatch {
        expected: ProducerIdentity,
        actual: ProducerIdentity,
    },
    #[error("producer result input fingerprints do not match the request precondition")]
    InputFingerprintMismatch,
    #[error("stale producer result has no changed evidence")]
    StaleWithoutChangedEvidence,
}

fn validate_output_evidence(
    request: &ProducerActionRequest,
    evidence: &ProducerActionEvidence,
) -> Result<(), ProducerActionResultError> {
    validate_observed_evidence(request, evidence)?;
    if evidence.input_fingerprints() != request.expected().input_fingerprints() {
        return Err(ProducerActionResultError::InputFingerprintMismatch);
    }
    Ok(())
}

fn validate_observed_evidence(
    request: &ProducerActionRequest,
    evidence: &ProducerActionEvidence,
) -> Result<(), ProducerActionResultError> {
    if evidence.producer() != request.producer() {
        return Err(ProducerActionResultError::ProducerIdentityMismatch {
            expected: request.producer().clone(),
            actual: evidence.producer().clone(),
        });
    }
    Ok(())
}
