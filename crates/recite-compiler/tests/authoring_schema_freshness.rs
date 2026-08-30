#![cfg(test)]

use recite_compiler::{
    SchemaFreshness, SchemaFreshnessEvidence, SchemaSummary, SchemaSummaryBuildError,
    SchemaSummaryEvidence, SchemaSummaryEvidenceError,
};
use recite_core::{
    ContentFingerprintFreshness, ProducerFingerprint, ProducerFreshness, ProjectSchema,
    SchemaProducerFreshness, load_schema_manifest_str,
};

const GENERATED: &str = include_str!("../../../fixtures/schema/valid/full_manifest.json");

fn generated_schema() -> ProjectSchema {
    load_schema_manifest_str("full_manifest.json", GENERATED)
        .schema
        .expect("generated fixture lowers")
}

#[test]
fn freshness_comparison_retains_simultaneous_stale_channels() {
    let expected = generated_schema();
    let mut actual = expected.clone();
    let producer = expected
        .producer_metadata
        .as_ref()
        .and_then(|metadata| metadata.producer.clone())
        .expect("producer identity");
    actual
        .producer_metadata
        .as_mut()
        .expect("producer metadata")
        .content_fingerprint = None;
    actual
        .producer_metadata
        .as_mut()
        .expect("producer metadata")
        .producer_fingerprints
        .clear();
    actual
        .registries
        .get_mut("item")
        .expect("registry")
        .producer_fingerprints
        .clear();
    if let Some(recite_core::MetadataDomainDefinition::Flat(domain)) =
        actual.metadata_domains.get_mut("tone")
    {
        domain.provenance.producer_fingerprints.clear();
    }

    let evidence = SchemaSummaryEvidence::builder(producer)
        .compare_freshness(&expected, &actual)
        .expect("matching snapshot identities")
        .build()
        .expect("freshness evidence");
    let summary = SchemaSummary::from_schema_with_evidence(&expected, Some(&evidence))
        .expect("freshness summary");
    let SchemaFreshness::Compared(freshness) = summary.freshness() else {
        panic!("comparison was not retained");
    };
    let SchemaProducerFreshness {
        content_fingerprint,
        manifest,
        registries,
        metadata_domains,
    } = freshness.as_ref();
    assert!(matches!(
        content_fingerprint,
        ContentFingerprintFreshness::Missing { .. }
    ));
    assert!(matches!(manifest, ProducerFreshness::Missing { .. }));
    assert!(matches!(
        registries["item"],
        ProducerFreshness::Missing { .. }
    ));
    assert!(matches!(
        metadata_domains["tone"],
        ProducerFreshness::Missing { .. }
    ));
}

#[test]
fn freshness_requires_both_snapshot_producer_identities() {
    let generated = generated_schema();
    let empty = ProjectSchema::empty_v1();
    assert!(matches!(
        SchemaFreshnessEvidence::from_snapshots(&empty, &generated),
        Err(SchemaSummaryEvidenceError::MissingSnapshotProducer { .. })
    ));
    assert!(matches!(
        SchemaFreshnessEvidence::from_snapshots(&generated, &empty),
        Err(SchemaSummaryEvidenceError::MissingSnapshotProducer { .. })
    ));
    let mut other = generated.clone();
    other
        .producer_metadata
        .as_mut()
        .expect("producer metadata")
        .producer = Some(recite_core::ProducerIdentity::new("adapter", "other").expect("identity"));
    assert!(matches!(
        SchemaFreshnessEvidence::from_snapshots(&generated, &other),
        Err(SchemaSummaryEvidenceError::ProducerIdentityMismatch { .. })
    ));
}

#[test]
fn freshness_evidence_cannot_be_attached_to_another_schema_with_same_producer() {
    let expected = generated_schema();
    let actual = expected.clone();
    let producer = expected
        .producer_metadata
        .as_ref()
        .and_then(|metadata| metadata.producer.clone())
        .expect("producer identity");
    let evidence = SchemaSummaryEvidence::builder(producer)
        .compare_freshness(&expected, &actual)
        .expect("matching snapshot identities")
        .build()
        .expect("bound freshness evidence");
    let mut unrelated = expected.clone();
    if let Some(recite_core::SchemaTypeDefinition::Enum(definition)) =
        unrelated.types.get_mut("mood")
    {
        definition.values.insert("unrelated".to_owned());
    }
    assert!(matches!(
        SchemaSummary::from_schema_with_evidence(&unrelated, Some(&evidence)),
        Err(SchemaSummaryBuildError::FreshnessSchemaMismatch { .. })
    ));
}

#[test]
fn mismatched_expected_fingerprint_is_rejected() {
    let expected = generated_schema();
    let mut actual = expected.clone();
    if let Some(recite_core::SchemaTypeDefinition::Enum(definition)) = actual.types.get_mut("mood")
    {
        definition.values.insert("current-only".to_owned());
    }
    let producer = expected
        .producer_metadata
        .as_ref()
        .and_then(|metadata| metadata.producer.clone())
        .expect("producer identity");
    let evidence = SchemaSummaryEvidence::builder(producer)
        .compare_freshness(&expected, &actual)
        .expect("matching snapshot identities")
        .build()
        .expect("bound freshness evidence");
    assert!(matches!(
        SchemaSummary::from_schema_with_evidence(&actual, Some(&evidence)),
        Err(SchemaSummaryBuildError::FreshnessSchemaMismatch { .. })
    ));
}

#[test]
fn producer_b_freshness_cannot_be_wrapped_as_producer_a() {
    let producer_a = generated_schema();
    let mut producer_b = producer_a.clone();
    producer_b
        .producer_metadata
        .as_mut()
        .expect("producer metadata")
        .producer =
        Some(recite_core::ProducerIdentity::new("adapter", "producer-b").expect("identity"));
    let producer_a_identity = producer_a
        .producer_metadata
        .as_ref()
        .and_then(|metadata| metadata.producer.clone())
        .expect("producer identity");
    assert!(matches!(
        SchemaSummaryEvidence::builder(producer_a_identity)
            .compare_freshness(&producer_b, &producer_b),
        Err(SchemaSummaryEvidenceError::ProducerIdentityMismatch { .. })
    ));
}

#[test]
fn freshness_identity_normalizes_input_order_without_dropping_entries() {
    let mut expected = generated_schema();
    let mut actual = expected.clone();
    let extra = ProducerFingerprint {
        id: "content/items/second".to_owned(),
        kind: "file".to_owned(),
        algorithm: "blake3".to_owned(),
        value: "second".to_owned(),
    };
    for schema in [&mut expected, &mut actual] {
        schema
            .producer_metadata
            .as_mut()
            .expect("producer metadata")
            .producer_fingerprints
            .extend([extra.clone(), extra.clone()]);
        schema
            .registries
            .get_mut("item")
            .expect("registry")
            .producer_fingerprints
            .extend([extra.clone(), extra.clone()]);
        if let Some(recite_core::MetadataDomainDefinition::Flat(domain)) =
            schema.metadata_domains.get_mut("tone")
        {
            domain
                .provenance
                .producer_fingerprints
                .extend([extra.clone(), extra.clone()]);
        }
    }
    expected
        .producer_metadata
        .as_mut()
        .expect("producer metadata")
        .producer_fingerprints
        .reverse();
    actual
        .producer_metadata
        .as_mut()
        .expect("producer metadata")
        .producer_fingerprints
        .reverse();
    expected
        .registries
        .get_mut("item")
        .expect("registry")
        .producer_fingerprints
        .reverse();
    actual
        .registries
        .get_mut("item")
        .expect("registry")
        .producer_fingerprints
        .reverse();
    let expected_domain = expected
        .metadata_domains
        .get_mut("tone")
        .expect("metadata domain");
    let actual_domain = actual
        .metadata_domains
        .get_mut("tone")
        .expect("metadata domain");
    if let (
        recite_core::MetadataDomainDefinition::Flat(expected_domain),
        recite_core::MetadataDomainDefinition::Flat(actual_domain),
    ) = (expected_domain, actual_domain)
    {
        expected_domain.provenance.producer_fingerprints.reverse();
        actual_domain.provenance.producer_fingerprints.reverse();
    }

    let evidence = SchemaFreshnessEvidence::from_snapshots(&expected, &actual)
        .expect("matching snapshot identities");
    assert_eq!(evidence.expected_identity(), evidence.actual_identity());
    assert_eq!(
        evidence
            .expected_identity()
            .manifest_producer_fingerprints()
            .iter()
            .filter(|fingerprint| fingerprint == &&extra)
            .count(),
        2
    );
}

#[test]
fn freshness_rejects_same_semantic_schema_with_different_content_metadata() {
    let expected = generated_schema();
    let mut actual = expected.clone();
    actual
        .producer_metadata
        .as_mut()
        .expect("producer metadata")
        .content_fingerprint = None;
    assert_eq!(
        expected.canonical_fingerprint(),
        actual.canonical_fingerprint()
    );
    let producer = expected
        .producer_metadata
        .as_ref()
        .and_then(|metadata| metadata.producer.clone())
        .expect("producer identity");
    let evidence = SchemaSummaryEvidence::builder(producer)
        .compare_freshness(&expected, &actual)
        .expect("matching snapshot identities")
        .build()
        .expect("bound freshness evidence");
    let freshness = evidence.freshness().expect("freshness evidence");
    assert_ne!(freshness.expected_identity(), freshness.actual_identity());
    assert!(matches!(
        SchemaSummary::from_schema_with_evidence(&actual, Some(&evidence)),
        Err(SchemaSummaryBuildError::FreshnessSchemaMismatch { .. })
    ));
}
