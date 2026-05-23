use super::CompiledValueError;

pub const BLAKE3_DIGEST_LEN: usize = 32;

#[must_use]
pub fn canonical_source_fingerprint(source: &str) -> ContentFingerprint {
    canonical_blake3_fingerprint(source.as_bytes())
}

#[must_use]
pub(crate) fn canonical_blake3_fingerprint(bytes: &[u8]) -> ContentFingerprint {
    let digest = blake3::hash(bytes);
    ContentFingerprint {
        algorithm: FingerprintAlgorithm::blake3(),
        digest: FingerprintDigest(digest.as_bytes().to_vec()),
    }
}

macro_rules! define_non_empty_string {
    ($name:ident) => {
        #[derive(Clone, Debug, Eq, PartialEq, Hash, Ord, PartialOrd)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, CompiledValueError> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(CompiledValueError::EmptyValue {
                        kind: stringify!($name),
                    });
                }

                Ok(Self(value))
            }

            #[must_use]
            pub fn as_str(&self) -> &str {
                &self.0
            }
        }
    };
}

define_non_empty_string!(FingerprintAlgorithm);

impl FingerprintAlgorithm {
    #[must_use]
    pub fn blake3() -> Self {
        Self("blake3".to_owned())
    }

    fn expected_digest_len(&self) -> Option<usize> {
        match self.as_str() {
            "blake3" => Some(BLAKE3_DIGEST_LEN),
            _ => None,
        }
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct FingerprintDigest(Vec<u8>);

impl FingerprintDigest {
    pub fn new(bytes: impl Into<Vec<u8>>) -> Result<Self, CompiledValueError> {
        let bytes = bytes.into();
        if bytes.is_empty() {
            return Err(CompiledValueError::EmptyValue {
                kind: "FingerprintDigest",
            });
        }

        Ok(Self(bytes))
    }

    #[must_use]
    pub fn as_bytes(&self) -> &[u8] {
        &self.0
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub struct ContentFingerprint {
    algorithm: FingerprintAlgorithm,
    digest: FingerprintDigest,
}

impl ContentFingerprint {
    pub fn new(
        algorithm: FingerprintAlgorithm,
        digest: FingerprintDigest,
    ) -> Result<Self, CompiledValueError> {
        if let Some(expected) = algorithm.expected_digest_len() {
            let actual = digest.as_bytes().len();
            if actual != expected {
                return Err(CompiledValueError::InvalidFingerprintDigestLength {
                    algorithm: "blake3",
                    expected,
                    actual,
                });
            }
        }

        Ok(Self { algorithm, digest })
    }

    pub fn blake3(digest: impl Into<Vec<u8>>) -> Result<Self, CompiledValueError> {
        Self::new(
            FingerprintAlgorithm::blake3(),
            FingerprintDigest::new(digest)?,
        )
    }

    #[must_use]
    pub fn algorithm(&self) -> &FingerprintAlgorithm {
        &self.algorithm
    }

    #[must_use]
    pub fn digest(&self) -> &FingerprintDigest {
        &self.digest
    }
}

#[derive(Clone, Debug, Eq, PartialEq, Hash)]
pub enum SchemaFingerprint {
    Fingerprint(ContentFingerprint),
    NoSchema,
}
