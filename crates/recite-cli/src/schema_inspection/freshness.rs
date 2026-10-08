use recite_compiler::authoring::SchemaFreshness;
use recite_core::schema::{ContentFingerprintFreshness, ProducerFreshness};

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
                status: crate::schema_freshness::freshness_status(comparison).to_owned(),
                reason: None,
                channels: Some(channels),
            }
        }
        SchemaFreshness::Unavailable { reason } => FreshnessProjection {
            status: "unavailable".to_owned(),
            reason: Some(match reason {
                recite_compiler::authoring::SchemaFreshnessUnavailableReason::NoComparisonSnapshot => {
                    "no_comparison_snapshot".to_owned()
                }
                recite_compiler::authoring::SchemaFreshnessUnavailableReason::NoProducerMetadata => {
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
