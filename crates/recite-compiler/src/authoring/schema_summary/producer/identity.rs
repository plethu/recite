use recite_core::{ContentFingerprint, ProducerIdentity};

use super::evidence::{ProducerActionEvidence, ProducerRetryGuidance};
use super::request::ProducerActionOperation;
use super::scopes::ProducerLaunchSnapshot;

/// Stable identity of a producer action request.
#[derive(Clone, Debug, Eq, Hash, PartialEq)]
#[non_exhaustive]
pub struct ProducerActionRequestIdentity(ContentFingerprint);

impl ProducerActionRequestIdentity {
    #[must_use]
    pub const fn fingerprint(&self) -> &ContentFingerprint {
        &self.0
    }

    pub(super) fn from_parts(
        producer: &ProducerIdentity,
        operation: &ProducerActionOperation,
        expected: &ProducerActionEvidence,
        launch: &ProducerLaunchSnapshot,
    ) -> Self {
        let mut bytes = Vec::from(&b"recite-producer-action-request-v2\0"[..]);
        write_identity(&mut bytes, producer);
        write_operation(&mut bytes, operation);
        write_evidence(&mut bytes, expected);
        write_identity(&mut bytes, launch.producer());
        write_scopes(&mut bytes, launch.input_fingerprints());
        Self(recite_core::canonical_source_fingerprint(&bytes_to_hex(
            &bytes,
        )))
    }
}

fn write_identity(bytes: &mut Vec<u8>, identity: &ProducerIdentity) {
    write_string(bytes, identity.kind());
    write_string(bytes, identity.id());
}

fn write_operation(bytes: &mut Vec<u8>, operation: &ProducerActionOperation) {
    match operation {
        ProducerActionOperation::Regenerate => bytes.push(0),
        ProducerActionOperation::Retry {
            failure,
            originating_request,
        } => {
            bytes.push(1);
            write_string(bytes, failure.code());
            write_optional_string(bytes, failure.detail());
            bytes.push(match failure.retry_guidance() {
                ProducerRetryGuidance::RetryNow => 0,
                ProducerRetryGuidance::RetryAfterCorrection => 1,
                ProducerRetryGuidance::DoNotRetry => 2,
            });
            write_fingerprint(bytes, originating_request.fingerprint());
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
    write_scopes(bytes, evidence.input_fingerprints());
    match evidence.output_fingerprint() {
        Some(fingerprint) => {
            bytes.push(1);
            write_fingerprint(bytes, fingerprint);
        }
        None => bytes.push(0),
    }
}

fn write_scopes(bytes: &mut Vec<u8>, scopes: &super::scopes::ProducerFingerprintScopes) {
    write_fingerprints(bytes, scopes.manifest());
    write_len(bytes, scopes.registries().len());
    for (name, fingerprints) in scopes.registries() {
        write_string(bytes, name);
        write_fingerprints(bytes, fingerprints);
    }
    write_len(bytes, scopes.metadata_domains().len());
    for (name, fingerprints) in scopes.metadata_domains() {
        write_string(bytes, name);
        write_fingerprints(bytes, fingerprints);
    }
}

fn write_fingerprints(bytes: &mut Vec<u8>, fingerprints: &[recite_core::ProducerFingerprint]) {
    write_len(bytes, fingerprints.len());
    for fingerprint in fingerprints {
        write_string(bytes, &fingerprint.kind);
        write_string(bytes, &fingerprint.id);
        write_string(bytes, &fingerprint.algorithm);
        write_string(bytes, &fingerprint.value);
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
