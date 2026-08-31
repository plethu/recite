use recite_core::ProducerIdentity;

use super::super::evidence::ProducerFailureEvidence;
use super::evidence::ProducerActionEvidence;
use super::identity::ProducerActionRequestIdentity;
use super::scopes::ProducerLaunchSnapshot;

/// The producer operation requested by a host client.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProducerActionOperation {
    Regenerate,
    Retry {
        failure: ProducerFailureEvidence,
        originating_request: ProducerActionRequestIdentity,
    },
}

/// A deterministic, data-only producer request bound to expected output and
/// the complete caller-supplied launch snapshot.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ProducerActionRequest {
    producer: ProducerIdentity,
    operation: ProducerActionOperation,
    expected: ProducerActionEvidence,
    launch: ProducerLaunchSnapshot,
    identity: ProducerActionRequestIdentity,
}

impl ProducerActionRequest {
    /// Build a request. Retry operations are intentionally private to failed
    /// result construction so a caller cannot fabricate their origin.
    pub fn new(
        producer: ProducerIdentity,
        operation: ProducerActionOperation,
        expected: ProducerActionEvidence,
        launch: ProducerLaunchSnapshot,
    ) -> Result<Self, ProducerActionRequestError> {
        Self::validate_common(&producer, &expected, &launch)?;
        if matches!(operation, ProducerActionOperation::Retry { .. }) {
            return Err(ProducerActionRequestError::RetryRequiresFailedResult);
        }
        Ok(Self::from_parts(producer, operation, expected, launch))
    }

    /// Construct a regeneration request from current expected output and
    /// current launch/preflight evidence.
    pub fn regenerate(
        expected: ProducerActionEvidence,
        launch: ProducerLaunchSnapshot,
    ) -> Result<Self, ProducerActionRequestError> {
        Self::new(
            launch.producer().clone(),
            ProducerActionOperation::Regenerate,
            expected,
            launch,
        )
    }

    pub(super) fn retry_from_failure(
        producer: ProducerIdentity,
        expected: ProducerActionEvidence,
        launch: ProducerLaunchSnapshot,
        failure: ProducerFailureEvidence,
        originating_request: ProducerActionRequestIdentity,
    ) -> Result<Self, ProducerActionRequestError> {
        Self::validate_common(&producer, &expected, &launch)?;
        if failure.producer() != &producer {
            return Err(ProducerActionRequestError::RetryFailureIdentityMismatch {
                expected: producer,
                actual: failure.producer().clone(),
            });
        }
        if !failure.retry_guidance().allows_retry() {
            return Err(ProducerActionRequestError::RetryNotAllowed);
        }
        Ok(Self::from_parts(
            producer,
            ProducerActionOperation::Retry {
                failure,
                originating_request,
            },
            expected,
            launch,
        ))
    }

    fn validate_common(
        producer: &ProducerIdentity,
        expected: &ProducerActionEvidence,
        launch: &ProducerLaunchSnapshot,
    ) -> Result<(), ProducerActionRequestError> {
        if producer != launch.producer() {
            return Err(ProducerActionRequestError::ProducerIdentityMismatch {
                expected: producer.clone(),
                actual: launch.producer().clone(),
            });
        }
        if producer != expected.producer() {
            return Err(ProducerActionRequestError::ProducerIdentityMismatch {
                expected: producer.clone(),
                actual: expected.producer().clone(),
            });
        }
        if expected.input_fingerprints() != launch.input_fingerprints() {
            return Err(ProducerActionRequestError::ExpectedInputMismatch);
        }
        Ok(())
    }

    fn from_parts(
        producer: ProducerIdentity,
        operation: ProducerActionOperation,
        expected: ProducerActionEvidence,
        launch: ProducerLaunchSnapshot,
    ) -> Self {
        let identity =
            ProducerActionRequestIdentity::from_parts(&producer, &operation, &expected, &launch);
        Self {
            producer,
            operation,
            expected,
            launch,
            identity,
        }
    }

    #[must_use]
    pub const fn producer(&self) -> &ProducerIdentity {
        &self.producer
    }

    #[must_use]
    pub const fn operation(&self) -> &ProducerActionOperation {
        &self.operation
    }

    #[must_use]
    pub const fn expected(&self) -> &ProducerActionEvidence {
        &self.expected
    }

    #[must_use]
    pub const fn launch_snapshot(&self) -> &ProducerLaunchSnapshot {
        &self.launch
    }

    #[must_use]
    pub const fn identity(&self) -> &ProducerActionRequestIdentity {
        &self.identity
    }
}

#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ProducerActionRequestError {
    #[error("producer request identity does not match supplied evidence")]
    ProducerIdentityMismatch {
        expected: ProducerIdentity,
        actual: ProducerIdentity,
    },
    #[error("producer expected evidence does not match launch inputs")]
    ExpectedInputMismatch,
    #[error("producer retry requests must be constructed from a validated failed result")]
    RetryRequiresFailedResult,
    #[error("producer retry requires a failed result")]
    NotFailedResult,
    #[error("producer retry failure does not match the request producer")]
    RetryFailureIdentityMismatch {
        expected: ProducerIdentity,
        actual: ProducerIdentity,
    },
    #[error("producer failure does not permit retry")]
    RetryNotAllowed,
}

/// A typed capability descriptor for a producer request.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ProducerActionDescriptor {
    request: ProducerActionRequest,
}

impl ProducerActionDescriptor {
    #[must_use]
    pub const fn new(request: ProducerActionRequest) -> Self {
        Self { request }
    }

    #[must_use]
    pub const fn request(&self) -> &ProducerActionRequest {
        &self.request
    }
}
