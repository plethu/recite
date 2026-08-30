#![cfg(test)]

use recite_compiler::{
    ProducerCapabilityStatus, ProducerFailureEvidence, SchemaAction, SchemaFreshness,
    SchemaFreshnessEvidence, SchemaSummary, SchemaSummaryBuildError, SchemaSummaryEvidence,
    SchemaSummaryEvidenceError,
};
use recite_core::{
    ContentFingerprintFreshness, ProducerFreshness, ProjectSchema, SchemaProducerFreshness,
    load_schema_manifest_str,
};

const GENERATED: &str = include_str!("../../../fixtures/schema/valid/full_manifest.json");

fn generated_schema() -> ProjectSchema {
    load_schema_manifest_str("full_manifest.json", GENERATED)
        .schema
        .expect("generated fixture lowers")
}

#[test]
fn supported_evidence_allows_invocation_and_only_current_failures_allow_retry() {
    let schema = generated_schema();
    let producer = schema
        .producer_metadata
        .as_ref()
        .and_then(|metadata| metadata.producer.clone())
        .expect("producer identity");
    let supported = SchemaSummaryEvidence::builder(producer.clone())
        .capability(ProducerCapabilityStatus::Supported)
        .build()
        .expect("supported evidence");
    let summary = SchemaSummary::from_schema_with_evidence(&schema, Some(&supported))
        .expect("matching evidence");
    assert!(
        summary
            .capability()
            .supports(&SchemaAction::InvokeProducer {
                producer: producer.clone(),
            })
    );
    assert!(
        !summary
            .capability()
            .actions()
            .iter()
            .any(|action| matches!(action, SchemaAction::RetryProducerFailure { .. }))
    );

    let failure = ProducerFailureEvidence::new(
        producer.clone(),
        "producer-exit",
        Some("producer returned a failure status".to_owned()),
    )
    .expect("failure evidence");
    let failed = SchemaSummaryEvidence::builder(producer.clone())
        .capability(ProducerCapabilityStatus::Supported)
        .current_failure(failure)
        .build()
        .expect("supported failure evidence");
    let failed_summary = SchemaSummary::from_schema_with_evidence(&schema, Some(&failed))
        .expect("matching failed evidence");
    assert!(
        failed_summary
            .capability()
            .supports(&SchemaAction::RetryProducerFailure { producer })
    );
}

#[test]
fn absent_read_only_and_unavailable_capabilities_are_distinct() {
    let schema = generated_schema();
    let producer = schema
        .producer_metadata
        .as_ref()
        .and_then(|metadata| metadata.producer.clone())
        .expect("producer identity");

    let default_summary = SchemaSummary::from_schema(&schema);
    assert!(
        default_summary
            .capability()
            .supports(&SchemaAction::ReadOnlyGenerated)
    );

    let unavailable = SchemaSummaryEvidence::builder(producer.clone())
        .capability(ProducerCapabilityStatus::Unavailable)
        .build()
        .expect("unavailable evidence");
    let unavailable_summary = SchemaSummary::from_schema_with_evidence(&schema, Some(&unavailable))
        .expect("matching unavailable evidence");
    assert!(
        unavailable_summary
            .capability()
            .actions()
            .iter()
            .any(|action| matches!(action, SchemaAction::Unavailable { .. }))
    );

    let other = recite_core::ProducerIdentity::new("adapter", "other").expect("identity");
    let mismatch = SchemaSummaryEvidence::builder(other.clone())
        .capability(ProducerCapabilityStatus::Supported)
        .build()
        .expect("independently valid evidence");
    assert!(matches!(
        SchemaSummary::from_schema_with_evidence(&schema, Some(&mismatch)),
        Err(SchemaSummaryBuildError::ProducerIdentityMismatch { .. })
    ));
    let mismatch_failure =
        ProducerFailureEvidence::new(other, "producer-exit", None).expect("failure evidence");
    assert!(matches!(
        SchemaSummaryEvidence::builder(producer)
            .capability(ProducerCapabilityStatus::ReadOnly)
            .current_failure(mismatch_failure)
            .build(),
        Err(SchemaSummaryEvidenceError::ProducerIdentityMismatch { .. })
    ));

    let matching_producer = schema
        .producer_metadata
        .as_ref()
        .and_then(|metadata| metadata.producer.clone())
        .expect("producer identity");
    let matching_failure =
        ProducerFailureEvidence::new(matching_producer.clone(), "producer-exit", None)
            .expect("failure evidence");
    assert!(matches!(
        SchemaSummaryEvidence::builder(matching_producer)
            .capability(ProducerCapabilityStatus::ReadOnly)
            .current_failure(matching_failure)
            .build(),
        Err(SchemaSummaryEvidenceError::ContradictoryStates)
    ));
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
    let freshness = SchemaFreshnessEvidence::from_snapshots(&expected, &actual)
        .expect("matching snapshot identities");
    let evidence = SchemaSummaryEvidence::builder(producer)
        .freshness(freshness)
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
    let freshness = SchemaFreshnessEvidence::from_snapshots(&expected, &actual)
        .expect("matching snapshot identities");
    let evidence = SchemaSummaryEvidence::builder(producer)
        .freshness(freshness)
        .build()
        .expect("bound freshness evidence");
    assert!(matches!(
        SchemaSummary::from_schema_with_evidence(&actual, Some(&evidence)),
        Err(SchemaSummaryBuildError::FreshnessSchemaMismatch { .. })
    ));
}
