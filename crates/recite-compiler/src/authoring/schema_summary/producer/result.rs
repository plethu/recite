use recite_core::ProducerIdentity;

use super::super::evidence::ProducerFailureEvidence;
use super::evidence::ProducerActionOutputEvidence;
use super::identity::ProducerActionRequestIdentity;
use super::request::{ProducerActionRequest, ProducerActionRequestError};
use super::scopes::ProducerLaunchSnapshot;

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
    Succeeded {
        evidence: ProducerActionOutputEvidence,
    },
    Failed {
        failure: ProducerFailureEvidence,
    },
    Cancelled,
    Stale {
        observed: ProducerLaunchSnapshot,
    },
}

/// A result bound to the complete request that created it.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ProducerActionResult {
    request: ProducerActionRequest,
    outcome: ProducerActionResultOutcome,
}

impl ProducerActionResult {
    /// Record reloaded output evidence. Input preconditions must be unchanged;
    /// output and schema fingerprints may legitimately be new.
    pub fn succeeded(
        request: &ProducerActionRequest,
        evidence: ProducerActionOutputEvidence,
    ) -> Result<Self, ProducerActionResultError> {
        validate_producer(request, evidence.producer())?;
        if evidence.input_fingerprints() != request.launch_snapshot().input_fingerprints() {
            return Err(ProducerActionResultError::InputFingerprintMismatch);
        }
        Ok(Self {
            request: request.clone(),
            outcome: ProducerActionResultOutcome::Succeeded { evidence },
        })
    }

    /// Record a structured producer failure.
    pub fn failed(
        request: &ProducerActionRequest,
        failure: ProducerFailureEvidence,
    ) -> Result<Self, ProducerActionResultError> {
        validate_producer(request, failure.producer())?;
        Ok(Self {
            request: request.clone(),
            outcome: ProducerActionResultOutcome::Failed { failure },
        })
    }

    /// Record a host cancellation without inventing a producer failure.
    #[must_use]
    pub fn cancelled(request: &ProducerActionRequest) -> Self {
        Self {
            request: request.clone(),
            outcome: ProducerActionResultOutcome::Cancelled,
        }
    }

    /// Record refusal because the caller observed changed launch inputs.
    pub fn stale(
        request: &ProducerActionRequest,
        observed: ProducerLaunchSnapshot,
    ) -> Result<Self, ProducerActionResultError> {
        validate_producer(request, observed.producer())?;
        if observed.input_fingerprints() == request.launch_snapshot().input_fingerprints() {
            return Err(ProducerActionResultError::StaleWithoutChangedInputs);
        }
        Ok(Self {
            request: request.clone(),
            outcome: ProducerActionResultOutcome::Stale { observed },
        })
    }

    #[must_use]
    pub const fn request(&self) -> &ProducerActionRequest {
        &self.request
    }

    #[must_use]
    pub const fn request_identity(&self) -> &ProducerActionRequestIdentity {
        self.request.identity()
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
    pub const fn evidence(&self) -> Option<&ProducerActionOutputEvidence> {
        match &self.outcome {
            ProducerActionResultOutcome::Succeeded { evidence } => Some(evidence),
            ProducerActionResultOutcome::Failed { .. }
            | ProducerActionResultOutcome::Cancelled
            | ProducerActionResultOutcome::Stale { .. } => None,
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
    pub const fn observed_stale_snapshot(&self) -> Option<&ProducerLaunchSnapshot> {
        match &self.outcome {
            ProducerActionResultOutcome::Stale { observed } => Some(observed),
            ProducerActionResultOutcome::Succeeded { .. }
            | ProducerActionResultOutcome::Failed { .. }
            | ProducerActionResultOutcome::Cancelled => None,
        }
    }

    /// Construct a retry only from this exact failed result.
    pub fn retry_request(&self) -> Result<ProducerActionRequest, ProducerActionRequestError> {
        let failure = self
            .failure()
            .ok_or(ProducerActionRequestError::NotFailedResult)?;
        ProducerActionRequest::retry_from_failure(
            self.request.producer().clone(),
            self.request.expected().clone(),
            self.request.launch_snapshot().clone(),
            failure.clone(),
            self.request.identity().clone(),
        )
    }

    /// Verify that this result belongs to the supplied request.
    pub fn validate_for(
        &self,
        request: &ProducerActionRequest,
    ) -> Result<(), ProducerActionResultError> {
        if self.request_identity() != request.identity() {
            return Err(ProducerActionResultError::RequestIdentityMismatch);
        }
        Ok(())
    }
}

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
    #[error("stale producer result has no changed launch inputs")]
    StaleWithoutChangedInputs,
}

fn validate_producer(
    request: &ProducerActionRequest,
    actual: &ProducerIdentity,
) -> Result<(), ProducerActionResultError> {
    if actual != request.producer() {
        return Err(ProducerActionResultError::ProducerIdentityMismatch {
            expected: request.producer().clone(),
            actual: actual.clone(),
        });
    }
    Ok(())
}
