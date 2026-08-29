use crate::{ContentFingerprint, FingerprintAlgorithm, FingerprintDigest};
use thiserror::Error;

#[derive(Clone, Debug, Eq, PartialEq, Error)]
pub(crate) enum ProducerContentFingerprintError {
    #[error("FingerprintAlgorithm must not be empty")]
    EmptyAlgorithm,
    #[error("blake3 producer fingerprint must be even-length hex")]
    Blake3HexShape,
    #[error("blake3 producer fingerprint must be hex")]
    Blake3HexData,
    #[error("FingerprintDigest must not be empty")]
    EmptyDigest,
    #[error("blake3 fingerprint digest must be 32 bytes, got {actual}")]
    Blake3DigestLength { actual: usize },
}

pub(crate) fn producer_content_fingerprint_detailed(
    algorithm: impl Into<String>,
    value: &str,
) -> Result<ContentFingerprint, ProducerContentFingerprintError> {
    let algorithm = FingerprintAlgorithm::new(algorithm)
        .map_err(|_| ProducerContentFingerprintError::EmptyAlgorithm)?;
    let digest = if algorithm.as_str() == "blake3" {
        if !value.len().is_multiple_of(2) || !value.is_ascii() {
            return Err(ProducerContentFingerprintError::Blake3HexShape);
        }
        (0..value.len())
            .step_by(2)
            .map(|index| u8::from_str_radix(&value[index..index + 2], 16))
            .collect::<Result<Vec<_>, _>>()
            .map_err(|_| ProducerContentFingerprintError::Blake3HexData)?
    } else {
        value.as_bytes().to_vec()
    };
    let digest =
        FingerprintDigest::new(digest).map_err(|_| ProducerContentFingerprintError::EmptyDigest)?;
    ContentFingerprint::new(algorithm, digest).map_err(|error| match error {
        crate::CompiledValueError::InvalidFingerprintDigestLength { actual, .. } => {
            ProducerContentFingerprintError::Blake3DigestLength { actual }
        }
        _ => unreachable!("algorithm and digest have already passed their validation"),
    })
}
