use std::collections::{BTreeMap, BTreeSet};

use crate::{ContentFingerprint, MetadataDomainDefinition};

use super::ProjectSchema;
use super::producer::{
    ProducerFingerprint, ProducerFreshness, ProducerMetadata, compare_producer_fingerprints,
};

/// Difference between optional manifest-level content fingerprints.
///
/// This is kept separate from producer input fingerprints because a content
/// digest describes the complete exported manifest, while producer input
/// fingerprints identify individual host inputs. In particular, the two
/// channels may legitimately use the same `kind` and `id` values.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum ContentFingerprintFreshness {
    Fresh,
    Missing {
        expected: ContentFingerprint,
    },
    Mismatch {
        expected: ContentFingerprint,
        actual: ContentFingerprint,
    },
    Unexpected {
        actual: ContentFingerprint,
    },
}

/// Deterministic freshness evidence for all producer-owned manifest channels.
///
/// Producer input fingerprints are compared independently at manifest,
/// registry, and metadata-domain scope. Scope names are retained as map keys
/// instead of being folded into synthetic producer fingerprint identities.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct SchemaProducerFreshness {
    pub content_fingerprint: ContentFingerprintFreshness,
    pub manifest: ProducerFreshness,
    pub registries: BTreeMap<String, ProducerFreshness>,
    pub metadata_domains: BTreeMap<String, ProducerFreshness>,
}

impl SchemaProducerFreshness {
    #[must_use]
    pub fn is_fresh(&self) -> bool {
        matches!(self.content_fingerprint, ContentFingerprintFreshness::Fresh)
            && matches!(self.manifest, ProducerFreshness::Fresh)
            && self
                .registries
                .values()
                .all(|freshness| matches!(freshness, ProducerFreshness::Fresh))
            && self
                .metadata_domains
                .values()
                .all(|freshness| matches!(freshness, ProducerFreshness::Fresh))
    }
}

/// Preserve the pre-1.0 producer-freshness result API.
///
/// Use [`compare_schema_producer_freshness_detailed`] when callers need the
/// typed content, registry, and metadata-domain channels introduced for the
/// freshness command. This compatibility result returns the first non-fresh
/// channel in deterministic manifest, registry, domain, then content order;
/// detailed results retain all channels when more than one is stale.
#[must_use]
pub fn compare_schema_producer_freshness(
    expected: &ProjectSchema,
    actual: &ProjectSchema,
) -> ProducerFreshness {
    let evidence = compare_schema_producer_freshness_detailed(expected, actual);
    if !matches!(evidence.manifest, ProducerFreshness::Fresh) {
        return evidence.manifest;
    }
    let producer_result = evidence
        .registries
        .values()
        .chain(evidence.metadata_domains.values())
        .find(|freshness| !matches!(freshness, ProducerFreshness::Fresh))
        .cloned();
    match producer_result {
        Some(producer_result) => producer_result,
        None => match evidence.content_fingerprint {
            ContentFingerprintFreshness::Fresh => ProducerFreshness::Fresh,
            ContentFingerprintFreshness::Missing { expected } => {
                ProducerFreshness::ContentMissing { expected }
            }
            ContentFingerprintFreshness::Mismatch { expected, actual } => {
                ProducerFreshness::ContentMismatch { expected, actual }
            }
            ContentFingerprintFreshness::Unexpected { actual } => {
                ProducerFreshness::ContentUnexpected { actual }
            }
        },
    }
}

/// Compare producer-owned freshness evidence from two manifests. Callers
/// supply both snapshots; this function never reads host resources.
#[must_use]
pub fn compare_schema_producer_freshness_detailed(
    expected: &ProjectSchema,
    actual: &ProjectSchema,
) -> SchemaProducerFreshness {
    let expected_metadata = expected.producer_metadata.as_ref();
    let actual_metadata = actual.producer_metadata.as_ref();
    SchemaProducerFreshness {
        content_fingerprint: compare_content_fingerprints(
            expected_metadata.and_then(|metadata| metadata.content_fingerprint.as_ref()),
            actual_metadata.and_then(|metadata| metadata.content_fingerprint.as_ref()),
        ),
        manifest: compare_producer_fingerprints(
            &manifest_fingerprints(expected_metadata),
            &manifest_fingerprints(actual_metadata),
        ),
        registries: compare_registry_fingerprints(expected, actual),
        metadata_domains: compare_domain_fingerprints(expected, actual),
    }
}

fn manifest_fingerprints(metadata: Option<&ProducerMetadata>) -> Vec<ProducerFingerprint> {
    metadata.map_or_else(Vec::new, |metadata| metadata.producer_fingerprints.clone())
}

fn compare_content_fingerprints(
    expected: Option<&ContentFingerprint>,
    actual: Option<&ContentFingerprint>,
) -> ContentFingerprintFreshness {
    match (expected, actual) {
        (None, None) => ContentFingerprintFreshness::Fresh,
        (Some(expected), None) => ContentFingerprintFreshness::Missing {
            expected: expected.clone(),
        },
        (Some(expected), Some(actual)) if expected != actual => {
            ContentFingerprintFreshness::Mismatch {
                expected: expected.clone(),
                actual: actual.clone(),
            }
        }
        (None, Some(actual)) => ContentFingerprintFreshness::Unexpected {
            actual: actual.clone(),
        },
        (Some(_), Some(_)) => ContentFingerprintFreshness::Fresh,
    }
}

fn compare_registry_fingerprints(
    expected: &ProjectSchema,
    actual: &ProjectSchema,
) -> BTreeMap<String, ProducerFreshness> {
    let names = expected
        .registries
        .keys()
        .chain(actual.registries.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    names
        .into_iter()
        .map(|name| {
            let expected_fingerprints =
                expected.registries.get(&name).map_or(&[][..], |registry| {
                    registry.producer_fingerprints.as_slice()
                });
            let actual_fingerprints = actual.registries.get(&name).map_or(&[][..], |registry| {
                registry.producer_fingerprints.as_slice()
            });
            (
                name,
                compare_producer_fingerprints(expected_fingerprints, actual_fingerprints),
            )
        })
        .collect()
}

fn compare_domain_fingerprints(
    expected: &ProjectSchema,
    actual: &ProjectSchema,
) -> BTreeMap<String, ProducerFreshness> {
    let names = expected
        .metadata_domains
        .keys()
        .chain(actual.metadata_domains.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    names
        .into_iter()
        .map(|name| {
            let expected_fingerprints = expected
                .metadata_domains
                .get(&name)
                .map_or(&[][..], domain_fingerprints);
            let actual_fingerprints = actual
                .metadata_domains
                .get(&name)
                .map_or(&[][..], domain_fingerprints);
            (
                name,
                compare_producer_fingerprints(expected_fingerprints, actual_fingerprints),
            )
        })
        .collect()
}

fn domain_fingerprints(domain: &MetadataDomainDefinition) -> &[ProducerFingerprint] {
    match domain {
        MetadataDomainDefinition::Flat(domain) => &domain.provenance.producer_fingerprints,
        MetadataDomainDefinition::Contextual(domain) => &domain.provenance.producer_fingerprints,
    }
}
