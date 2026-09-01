use std::collections::BTreeMap;

use crate::ContentFingerprint;
use serde_json::Value;

mod fingerprint;
mod identity;

pub(crate) use fingerprint::{
    ProducerContentFingerprintError, producer_content_fingerprint_detailed,
};
pub use identity::{ProducerIdentity, ProducerIdentityError, ProducerIdentityPart};

/// Format-neutral recursive value for producer diagnostic extensions.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProducerMetadataValue {
    Null,
    Bool(bool),
    Number(String),
    String(String),
    Array(Vec<Self>),
    Object(BTreeMap<String, Self>),
}

impl ProducerMetadataValue {
    pub(crate) fn from_json(value: Value) -> Self {
        match value {
            Value::Null => Self::Null,
            Value::Bool(value) => Self::Bool(value),
            Value::Number(value) => Self::Number(value.to_string()),
            Value::String(value) => Self::String(value),
            Value::Array(values) => Self::Array(values.into_iter().map(Self::from_json).collect()),
            Value::Object(values) => Self::Object(
                values
                    .into_iter()
                    .map(|(key, value)| (key, Self::from_json(value)))
                    .collect(),
            ),
        }
    }
}

/// Parse a producer-owned content digest through the historical string error surface.
pub fn producer_content_fingerprint(
    algorithm: impl Into<String>,
    value: &str,
) -> Result<ContentFingerprint, String> {
    producer_content_fingerprint_detailed(algorithm, value).map_err(|error| error.to_string())
}

/// Stable host-side provenance for a generated schema definition or value.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ProducerOrigin {
    pub kind: String,
    pub id: String,
    pub label: Option<String>,
    pub extensions: BTreeMap<String, ProducerMetadataValue>,
}

/// Repeatable fingerprint for the producer input set represented by a schema
/// manifest or one of its domains.
#[derive(Clone, Debug, Eq, Hash, Ord, PartialEq, PartialOrd)]
pub struct ProducerFingerprint {
    pub id: String,
    pub kind: String,
    pub algorithm: String,
    pub value: String,
}

/// Difference between expected and current producer content fingerprints.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProducerFingerprintMismatch {
    pub expected: ProducerFingerprint,
    pub actual: ProducerFingerprint,
}

/// Deterministic evidence from comparing producer-owned content snapshots.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ProducerFreshness {
    Fresh,
    ContentMissing {
        expected: ContentFingerprint,
    },
    ContentMismatch {
        expected: ContentFingerprint,
        actual: ContentFingerprint,
    },
    ContentUnexpected {
        actual: ContentFingerprint,
    },
    Invalid {
        expected_duplicates: Vec<(String, String)>,
        actual_duplicates: Vec<(String, String)>,
    },
    Missing {
        expected: Vec<ProducerFingerprint>,
    },
    Mismatch {
        entries: Vec<ProducerFingerprintMismatch>,
    },
    Unexpected {
        actual: Vec<ProducerFingerprint>,
    },
    Mixed {
        missing: Vec<ProducerFingerprint>,
        mismatched: Vec<ProducerFingerprintMismatch>,
        unexpected: Vec<ProducerFingerprint>,
    },
}

/// Compare producer content fingerprints without consulting host resources.
#[must_use]
pub fn compare_producer_fingerprints(
    expected: &[ProducerFingerprint],
    actual: &[ProducerFingerprint],
) -> ProducerFreshness {
    let expected_duplicates = duplicate_fingerprint_keys(expected);
    let actual_duplicates = duplicate_fingerprint_keys(actual);
    if !expected_duplicates.is_empty() || !actual_duplicates.is_empty() {
        return ProducerFreshness::Invalid {
            expected_duplicates,
            actual_duplicates,
        };
    }
    let expected_by_key = fingerprint_map(expected);
    let actual_by_key = fingerprint_map(actual);
    let missing = expected_by_key
        .iter()
        .filter_map(|(key, fingerprint)| {
            (!actual_by_key.contains_key(key)).then_some(fingerprint.clone())
        })
        .collect::<Vec<_>>();
    let mismatched = expected_by_key
        .iter()
        .filter_map(|(key, expected)| {
            actual_by_key.get(key).and_then(|actual| {
                (expected != actual).then_some(ProducerFingerprintMismatch {
                    expected: expected.clone(),
                    actual: actual.clone(),
                })
            })
        })
        .collect::<Vec<_>>();
    let unexpected = actual_by_key
        .iter()
        .filter_map(|(key, fingerprint)| {
            (!expected_by_key.contains_key(key)).then_some(fingerprint.clone())
        })
        .collect::<Vec<_>>();
    match (
        missing.is_empty(),
        mismatched.is_empty(),
        unexpected.is_empty(),
    ) {
        (true, true, true) => ProducerFreshness::Fresh,
        (false, true, true) => ProducerFreshness::Missing { expected: missing },
        (true, false, true) => ProducerFreshness::Mismatch {
            entries: mismatched,
        },
        (true, true, false) => ProducerFreshness::Unexpected { actual: unexpected },
        _ => ProducerFreshness::Mixed {
            missing,
            mismatched,
            unexpected,
        },
    }
}

fn duplicate_fingerprint_keys(fingerprints: &[ProducerFingerprint]) -> Vec<(String, String)> {
    let mut counts = BTreeMap::<(String, String), usize>::new();
    for fingerprint in fingerprints {
        let key = (fingerprint.kind.clone(), fingerprint.id.clone());
        *counts.entry(key).or_default() += 1;
    }
    counts
        .into_iter()
        .filter_map(|(key, count)| (count > 1).then_some(key))
        .collect()
}
type FingerprintMap = BTreeMap<(String, String), ProducerFingerprint>;

fn fingerprint_map(items: &[ProducerFingerprint]) -> FingerprintMap {
    let entries = items
        .iter()
        .map(|fp| ((fp.kind.clone(), fp.id.clone()), fp.clone()));
    entries.collect()
}

/// Non-semantic producer metadata retained for diagnostics and stale-output
/// checks.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProducerMetadata {
    pub producer: Option<ProducerIdentity>,
    pub content_fingerprint: Option<ContentFingerprint>,
    pub schema_export_version: Option<u32>,
    pub inclusion_policy: Option<String>,
    pub producer_fingerprints: Vec<ProducerFingerprint>,
}

/// Provenance and producer fingerprints attached to a flat metadata domain.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct FlatMetadataProvenance {
    pub origin: Option<ProducerOrigin>,
    pub value_origins: BTreeMap<String, ProducerOrigin>,
    pub producer_fingerprints: Vec<ProducerFingerprint>,
}

/// Provenance and producer fingerprints attached to a contextual metadata
/// domain and its context/value snapshots.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct ContextualMetadataProvenance {
    pub origin: Option<ProducerOrigin>,
    pub context_origins: BTreeMap<String, ProducerOrigin>,
    pub value_origins: BTreeMap<String, BTreeMap<String, ProducerOrigin>>,
    pub producer_fingerprints: Vec<ProducerFingerprint>,
}
