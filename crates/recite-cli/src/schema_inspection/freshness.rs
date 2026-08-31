use recite_compiler::SchemaFreshness;
use recite_core::{ContentFingerprintFreshness, ProducerFreshness, SchemaProducerFreshness};

use super::fingerprints::producer_fingerprint_projection;
use super::model::{FreshnessChannelsProjection, FreshnessProjection};

pub(super) fn freshness_json(freshness: &SchemaFreshness) -> FreshnessProjection {
    match freshness {
        SchemaFreshness::Compared(evidence) => {
            let comparison = evidence.as_ref();
            let channels = FreshnessChannelsProjection {
                content: content_freshness_json(&comparison.content_fingerprint),
                manifest: producer_freshness_json(&comparison.manifest),
                registries: comparison
                    .registries
                    .iter()
                    .map(|(name, value)| (name.clone(), producer_freshness_json(value)))
                    .collect(),
                metadata_domains: comparison
                    .metadata_domains
                    .iter()
                    .map(|(name, value)| (name.clone(), producer_freshness_json(value)))
                    .collect(),
            };
            FreshnessProjection {
                status: freshness_status(comparison),
                reason: None,
                channels: Some(channels),
            }
        }
        SchemaFreshness::Unavailable { reason } => FreshnessProjection {
            status: "unavailable".to_owned(),
            reason: Some(match reason {
                recite_compiler::SchemaFreshnessUnavailableReason::NoComparisonSnapshot => {
                    "no_comparison_snapshot".to_owned()
                }
                recite_compiler::SchemaFreshnessUnavailableReason::NoProducerMetadata => {
                    "no_producer_metadata".to_owned()
                }
                _ => "unknown".to_owned(),
            }),
            channels: None,
        },
        _ => FreshnessProjection {
            status: "unavailable".to_owned(),
            reason: Some("unknown".to_owned()),
            channels: None,
        },
    }
}

fn freshness_status(evidence: &SchemaProducerFreshness) -> String {
    let mut invalid = false;
    let mut missing = false;
    let mut mismatch = false;
    let mut unexpected = false;
    let mut record = |freshness: &ProducerFreshness| match freshness {
        ProducerFreshness::Fresh => {}
        ProducerFreshness::ContentMissing { .. } | ProducerFreshness::Missing { .. } => {
            missing = true
        }
        ProducerFreshness::ContentMismatch { .. } | ProducerFreshness::Mismatch { .. } => {
            mismatch = true
        }
        ProducerFreshness::ContentUnexpected { .. } | ProducerFreshness::Unexpected { .. } => {
            unexpected = true
        }
        ProducerFreshness::Invalid { .. } => invalid = true,
        ProducerFreshness::Mixed { .. } => {
            missing = true;
            mismatch = true;
            unexpected = true;
        }
    };
    record(&evidence.manifest);
    for value in evidence
        .registries
        .values()
        .chain(evidence.metadata_domains.values())
    {
        record(value);
    }
    match evidence.content_fingerprint {
        ContentFingerprintFreshness::Fresh => {}
        ContentFingerprintFreshness::Missing { .. } => missing = true,
        ContentFingerprintFreshness::Mismatch { .. } => mismatch = true,
        ContentFingerprintFreshness::Unexpected { .. } => unexpected = true,
    }
    if invalid {
        "invalid"
    } else if [missing, mismatch, unexpected]
        .into_iter()
        .filter(|value| *value)
        .count()
        > 1
    {
        "mixed"
    } else if missing {
        "missing"
    } else if mismatch {
        "mismatch"
    } else if unexpected {
        "unexpected"
    } else {
        "fresh"
    }
    .to_owned()
}

fn content_freshness_json(value: &ContentFingerprintFreshness) -> serde_json::Value {
    match value {
        ContentFingerprintFreshness::Fresh => serde_json::json!({ "status": "fresh" }),
        ContentFingerprintFreshness::Missing { expected } => {
            serde_json::json!({ "status": "missing", "expected": super::fingerprints::content_fingerprint_json(expected) })
        }
        ContentFingerprintFreshness::Mismatch { expected, actual } => serde_json::json!({
            "status": "mismatch",
            "expected": super::fingerprints::content_fingerprint_json(expected),
            "actual": super::fingerprints::content_fingerprint_json(actual),
        }),
        ContentFingerprintFreshness::Unexpected { actual } => {
            serde_json::json!({ "status": "unexpected", "actual": super::fingerprints::content_fingerprint_json(actual) })
        }
    }
}

fn producer_freshness_json(value: &ProducerFreshness) -> serde_json::Value {
    match value {
        ProducerFreshness::Fresh => serde_json::json!({ "status": "fresh" }),
        ProducerFreshness::ContentMissing { expected } => {
            serde_json::json!({ "status": "missing", "expected": super::fingerprints::content_fingerprint_json(expected) })
        }
        ProducerFreshness::ContentMismatch { expected, actual } => serde_json::json!({
            "status": "mismatch",
            "expected": super::fingerprints::content_fingerprint_json(expected),
            "actual": super::fingerprints::content_fingerprint_json(actual),
        }),
        ProducerFreshness::ContentUnexpected { actual } => {
            serde_json::json!({ "status": "unexpected", "actual": super::fingerprints::content_fingerprint_json(actual) })
        }
        ProducerFreshness::Invalid {
            expected_duplicates,
            actual_duplicates,
        } => serde_json::json!({
            "status": "invalid",
            "expected_duplicates": expected_duplicates,
            "actual_duplicates": actual_duplicates,
        }),
        ProducerFreshness::Missing { expected } => serde_json::json!({
            "status": "missing",
            "expected": expected.iter().map(producer_fingerprint_projection).collect::<Vec<_>>(),
        }),
        ProducerFreshness::Mismatch { entries } => serde_json::json!({
            "status": "mismatch",
            "entries": entries.iter().map(|entry| serde_json::json!({
                "expected": producer_fingerprint_projection(&entry.expected),
                "actual": producer_fingerprint_projection(&entry.actual),
            })).collect::<Vec<_>>(),
        }),
        ProducerFreshness::Unexpected { actual } => serde_json::json!({
            "status": "unexpected",
            "actual": actual.iter().map(producer_fingerprint_projection).collect::<Vec<_>>(),
        }),
        ProducerFreshness::Mixed {
            missing,
            mismatched,
            unexpected,
        } => serde_json::json!({
            "status": "mixed",
            "missing": missing.iter().map(producer_fingerprint_projection).collect::<Vec<_>>(),
            "mismatched": mismatched.iter().map(|entry| serde_json::json!({
                "expected": producer_fingerprint_projection(&entry.expected),
                "actual": producer_fingerprint_projection(&entry.actual),
            })).collect::<Vec<_>>(),
            "unexpected": unexpected.iter().map(producer_fingerprint_projection).collect::<Vec<_>>(),
        }),
    }
}
