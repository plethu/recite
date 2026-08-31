#![cfg(test)]

use recite_compiler::{
    ProducerCapabilityStatus, ProducerFailureEvidence, SchemaAction, SchemaSummary,
    SchemaSummaryBuildError, SchemaSummaryEvidence, SchemaSummaryEvidenceError,
};
use recite_core::{ProjectSchema, load_schema_manifest_str};

const GENERATED: &str = include_str!("../../../fixtures/schema/valid/full_manifest.json");

fn generated_schema() -> ProjectSchema {
    load_schema_manifest_str("full_manifest.json", GENERATED)
        .schema
        .expect("generated fixture lowers")
}

#[test]
fn supported_evidence_allows_invocation_and_bare_failures_do_not_allow_retry() {
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
        !failed_summary
            .capability()
            .actions()
            .iter()
            .any(|action| matches!(action, SchemaAction::RetryProducerFailure { .. }))
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
