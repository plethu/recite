#![cfg(test)]

use recite_compiler::{
    ProducerActionEvidence, ProducerActionOperation, ProducerActionRequest,
    ProducerActionRequestError, ProducerActionResult, ProducerActionResultError,
    ProducerActionStatus, ProducerCapabilityStatus, ProducerFailureEvidence, ProducerRetryGuidance,
    SchemaSummary, SchemaSummaryEvidence,
};
use recite_core::{
    ContentFingerprint, ProducerFingerprint, ProducerIdentity, ProjectSchema, SchemaFingerprint,
    load_schema_manifest_str,
};

const GENERATED: &str = include_str!("../../../fixtures/schema/valid/full_manifest.json");

fn generated_schema() -> ProjectSchema {
    load_schema_manifest_str("full_manifest.json", GENERATED)
        .schema
        .expect("generated fixture lowers")
}

fn generated_evidence() -> ProducerActionEvidence {
    SchemaSummary::from_schema(&generated_schema())
        .producer_action_evidence()
        .expect("generated fixture has producer evidence")
}

fn alternate_inputs() -> Vec<ProducerFingerprint> {
    vec![ProducerFingerprint {
        id: "content/other".to_owned(),
        kind: "directory".to_owned(),
        algorithm: "blake3".to_owned(),
        value: "different".to_owned(),
    }]
}

fn changed_inputs(evidence: &ProducerActionEvidence) -> ProducerActionEvidence {
    ProducerActionEvidence::new(
        evidence.producer().clone(),
        evidence.schema_fingerprint().clone(),
        evidence.content_fingerprint().clone(),
        alternate_inputs(),
        evidence.output_fingerprint().cloned(),
    )
}

fn changed_output(evidence: &ProducerActionEvidence) -> ProducerActionEvidence {
    ProducerActionEvidence::new(
        evidence.producer().clone(),
        evidence.schema_fingerprint().clone(),
        evidence.content_fingerprint().clone(),
        evidence.input_fingerprints().iter().cloned(),
        Some(ContentFingerprint::blake3(vec![9; 32]).expect("valid digest")),
    )
}

#[test]
fn request_identity_is_deterministic_and_retry_is_bound_to_prior_failure() {
    let expected = generated_evidence();
    let first = ProducerActionRequest::regenerate(expected.clone()).expect("request");
    let second = ProducerActionRequest::regenerate(expected.clone()).expect("request");
    assert_eq!(first.identity(), second.identity());

    let changed = ProducerActionEvidence::new(
        expected.producer().clone(),
        expected.schema_fingerprint().clone(),
        ContentFingerprint::blake3(vec![8; 32]).expect("valid digest"),
        expected.input_fingerprints().iter().cloned(),
        expected.output_fingerprint().cloned(),
    );
    let changed_request = ProducerActionRequest::regenerate(changed).expect("request");
    assert_ne!(first.identity(), changed_request.identity());

    let failure = ProducerFailureEvidence::new(
        expected.producer().clone(),
        "producer-exit",
        Some("producer returned a failure status".to_owned()),
    )
    .expect("failure")
    .with_retry_guidance(ProducerRetryGuidance::RetryAfterCorrection);
    let retry = ProducerActionRequest::retry(expected.clone(), failure.clone()).expect("retry");
    assert!(matches!(
        retry.operation(),
        ProducerActionOperation::Retry { failure: bound } if bound == &failure
    ));

    let other = ProducerIdentity::new("adapter", "other").expect("identity");
    let wrong_failure =
        ProducerFailureEvidence::new(other, "producer-exit", None).expect("failure");
    assert!(matches!(
        ProducerActionRequest::retry(expected, wrong_failure),
        Err(ProducerActionRequestError::RetryFailureIdentityMismatch { .. })
    ));
}

#[test]
fn result_rejects_wrong_request_identity_and_represents_cancellation() {
    let expected = generated_evidence();
    let request = ProducerActionRequest::regenerate(expected.clone()).expect("request");
    let changed = ProducerActionEvidence::new(
        expected.producer().clone(),
        expected.schema_fingerprint().clone(),
        ContentFingerprint::blake3(vec![7; 32]).expect("valid digest"),
        expected.input_fingerprints().iter().cloned(),
        expected.output_fingerprint().cloned(),
    );
    let other_request = ProducerActionRequest::regenerate(changed).expect("request");

    let result = ProducerActionResult::cancelled(&request);
    assert_eq!(result.status(), ProducerActionStatus::Cancelled);
    assert!(matches!(
        result.validate_for(&other_request),
        Err(ProducerActionResultError::RequestIdentityMismatch)
    ));
    result.validate_for(&request).expect("matching request");
}

#[test]
fn success_requires_current_inputs_but_returns_reloaded_output_evidence() {
    let expected = generated_evidence();
    let request = ProducerActionRequest::regenerate(expected.clone()).expect("request");

    let drifted = changed_inputs(&expected);
    assert!(matches!(
        ProducerActionResult::succeeded(&request, drifted),
        Err(ProducerActionResultError::InputFingerprintMismatch)
    ));

    let reloaded = changed_output(&expected);
    let result = ProducerActionResult::succeeded(&request, reloaded.clone()).expect("success");
    assert_eq!(result.status(), ProducerActionStatus::Succeeded);
    assert_eq!(result.evidence(), Some(&reloaded));
    assert!(result.failure().is_none());
}

#[test]
fn stale_and_failure_results_are_structured_and_retry_guidance_is_typed() {
    let expected = generated_evidence();
    let request = ProducerActionRequest::regenerate(expected.clone()).expect("request");
    let observed = changed_inputs(&expected);
    let stale = ProducerActionResult::stale(&request, observed.clone()).expect("stale");
    assert_eq!(stale.status(), ProducerActionStatus::Stale);
    assert_eq!(stale.observed_stale_evidence(), Some(&observed));
    assert!(matches!(
        ProducerActionResult::stale(&request, expected),
        Err(ProducerActionResultError::StaleWithoutChangedEvidence)
    ));

    let failure = ProducerFailureEvidence::new(
        request.producer().clone(),
        "producer-input-invalid",
        Some("input requires correction".to_owned()),
    )
    .expect("failure")
    .with_retry_guidance(ProducerRetryGuidance::RetryAfterCorrection);
    assert!(failure.retry_guidance().allows_retry());
    let result = ProducerActionResult::failed(&request, failure.clone()).expect("failure result");
    assert_eq!(result.status(), ProducerActionStatus::Failed);
    assert_eq!(result.failure(), Some(&failure));

    let no_retry = failure.with_retry_guidance(ProducerRetryGuidance::DoNotRetry);
    assert!(matches!(
        ProducerActionRequest::retry(generated_evidence(), no_retry),
        Err(ProducerActionRequestError::RetryNotAllowed)
    ));
}

#[test]
fn generated_capability_exposes_typed_descriptors_without_execution() {
    let schema = generated_schema();
    let producer = schema
        .producer_metadata
        .as_ref()
        .and_then(|metadata| metadata.producer.clone())
        .expect("producer identity");
    let failure =
        ProducerFailureEvidence::new(producer.clone(), "producer-exit", None).expect("failure");
    let evidence = SchemaSummaryEvidence::builder(producer)
        .capability(ProducerCapabilityStatus::Supported)
        .current_failure(failure)
        .build()
        .expect("evidence");
    let summary =
        SchemaSummary::from_schema_with_evidence(&schema, Some(&evidence)).expect("summary");
    let actions = summary.capability().producer_actions();
    assert_eq!(actions.len(), 2);
    assert!(matches!(
        actions[0].request().operation(),
        ProducerActionOperation::Regenerate
    ));
    assert!(matches!(
        actions[1].request().operation(),
        ProducerActionOperation::Retry { .. }
    ));
    assert_eq!(
        actions[0].request().expected().schema_fingerprint(),
        &SchemaFingerprint::Fingerprint(
            actions[0]
                .request()
                .expected()
                .content_fingerprint()
                .clone()
        )
    );
}
