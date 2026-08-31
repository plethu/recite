use recite_core::{ContentFingerprint, ProducerIdentity};

use super::super::evidence::ProducerFailureEvidence;
use super::evidence::{ProducerActionEvidence, ProducerRetryGuidance};

/// The producer operation requested by a host client.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub enum ProducerActionOperation {
    Regenerate,
    Retry { failure: ProducerFailureEvidence },
}

/// A deterministic request to a producer. It is a descriptor only: execution
/// remains the responsibility of the host client.
#[derive(Clone, Debug, Eq, PartialEq)]
#[non_exhaustive]
pub struct ProducerActionRequest {
    producer: ProducerIdentity,
    operation: ProducerActionOperation,
    expected: ProducerActionEvidence,
    identity: ProducerActionRequestIdentity,
}

impl ProducerActionRequest {
    /// Build a request after binding the expected evidence and, for retries,
    /// the prior failure to the same producer identity.
    pub fn new(
        producer: ProducerIdentity,
        operation: ProducerActionOperation,
        expected: ProducerActionEvidence,
    ) -> Result<Self, ProducerActionRequestError> {
        if producer != *expected.producer() {
            return Err(ProducerActionRequestError::ProducerIdentityMismatch {
                expected: producer,
                actual: expected.producer().clone(),
            });
        }
        if let ProducerActionOperation::Retry { failure } = &operation {
            if failure.producer() != &producer {
                return Err(ProducerActionRequestError::RetryFailureIdentityMismatch {
                    expected: producer,
                    actual: failure.producer().clone(),
                });
            }
            if !failure.retry_guidance().allows_retry() {
                return Err(ProducerActionRequestError::RetryNotAllowed);
            }
        }
        let identity = ProducerActionRequestIdentity::from_parts(&producer, &operation, &expected);
        Ok(Self {
            producer,
            operation,
            expected,
            identity,
        })
    }

    /// Construct a regeneration request from one evidence snapshot.
    pub fn regenerate(
        expected: ProducerActionEvidence,
    ) -> Result<Self, ProducerActionRequestError> {
        Self::new(
            expected.producer().clone(),
            ProducerActionOperation::Regenerate,
            expected,
        )
    }

    /// Construct a retry request that is explicitly bound to a prior failure.
    pub fn retry(
        expected: ProducerActionEvidence,
        failure: ProducerFailureEvidence,
    ) -> Result<Self, ProducerActionRequestError> {
        Self::new(
            expected.producer().clone(),
            ProducerActionOperation::Retry { failure },
            expected,
        )
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
    pub const fn identity(&self) -> &ProducerActionRequestIdentity {
        &self.identity
    }
}

/// Failure while constructing a producer action request.
#[derive(Clone, Debug, Eq, PartialEq, thiserror::Error)]
#[non_exhaustive]
pub enum ProducerActionRequestError {
    #[error("producer request identity does not match expected evidence")]
    ProducerIdentityMismatch {
        expected: ProducerIdentity,
        actual: ProducerIdentity,
    },
    #[error("producer retry failure does not match the request producer")]
    RetryFailureIdentityMismatch {
        expected: ProducerIdentity,
        actual: ProducerIdentity,
    },
    #[error("producer failure does not permit retry")]
    RetryNotAllowed,
}

/// Stable identity of a producer action request.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub struct ProducerActionRequestIdentity(ContentFingerprint);

impl ProducerActionRequestIdentity {
    #[must_use]
    pub const fn fingerprint(&self) -> &ContentFingerprint {
        &self.0
    }

    fn from_parts(
        producer: &ProducerIdentity,
        operation: &ProducerActionOperation,
        expected: &ProducerActionEvidence,
    ) -> Self {
        let mut bytes = Vec::from(&b"recite-producer-action-request-v1\0"[..]);
        write_identity(&mut bytes, producer);
        write_operation(&mut bytes, operation);
        write_evidence(&mut bytes, expected);
        let encoded = bytes_to_hex(&bytes);
        Self(recite_core::canonical_source_fingerprint(&encoded))
    }
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

fn write_identity(bytes: &mut Vec<u8>, identity: &ProducerIdentity) {
    write_string(bytes, identity.kind());
    write_string(bytes, identity.id());
}

fn write_operation(bytes: &mut Vec<u8>, operation: &ProducerActionOperation) {
    match operation {
        ProducerActionOperation::Regenerate => bytes.push(0),
        ProducerActionOperation::Retry { failure } => {
            bytes.push(1);
            write_string(bytes, failure.code());
            write_optional_string(bytes, failure.detail());
            bytes.push(match failure.retry_guidance() {
                ProducerRetryGuidance::RetryNow => 0,
                ProducerRetryGuidance::RetryAfterCorrection => 1,
                ProducerRetryGuidance::DoNotRetry => 2,
            });
        }
    }
}

fn write_evidence(bytes: &mut Vec<u8>, evidence: &ProducerActionEvidence) {
    write_identity(bytes, evidence.producer());
    match evidence.schema_fingerprint() {
        recite_core::SchemaFingerprint::NoSchema => bytes.push(0),
        recite_core::SchemaFingerprint::Fingerprint(fingerprint) => {
            bytes.push(1);
            write_fingerprint(bytes, fingerprint);
        }
    }
    write_fingerprint(bytes, evidence.content_fingerprint());
    write_len(bytes, evidence.input_fingerprints().len());
    for fingerprint in evidence.input_fingerprints() {
        write_string(bytes, &fingerprint.kind);
        write_string(bytes, &fingerprint.id);
        write_string(bytes, &fingerprint.algorithm);
        write_string(bytes, &fingerprint.value);
    }
    match evidence.output_fingerprint() {
        Some(fingerprint) => {
            bytes.push(1);
            write_fingerprint(bytes, fingerprint);
        }
        None => bytes.push(0),
    }
}

fn write_fingerprint(bytes: &mut Vec<u8>, fingerprint: &ContentFingerprint) {
    write_string(bytes, fingerprint.algorithm().as_str());
    write_len(bytes, fingerprint.digest().as_bytes().len());
    bytes.extend_from_slice(fingerprint.digest().as_bytes());
}

fn write_optional_string(bytes: &mut Vec<u8>, value: Option<&str>) {
    match value {
        Some(value) => {
            bytes.push(1);
            write_string(bytes, value);
        }
        None => bytes.push(0),
    }
}

fn write_string(bytes: &mut Vec<u8>, value: &str) {
    write_len(bytes, value.len());
    bytes.extend_from_slice(value.as_bytes());
}

fn write_len(bytes: &mut Vec<u8>, value: usize) {
    bytes.extend_from_slice(&(value as u64).to_be_bytes());
}

fn bytes_to_hex(bytes: &[u8]) -> String {
    const HEX: &[u8; 16] = b"0123456789abcdef";
    let mut encoded = String::with_capacity(bytes.len() * 2);
    for byte in bytes {
        encoded.push(HEX[(byte >> 4) as usize] as char);
        encoded.push(HEX[(byte & 0x0f) as usize] as char);
    }
    encoded
}
