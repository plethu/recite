#![cfg(test)]

use recite_compiler::{
    ProducerActionEvidence, ProducerActionOperation, ProducerActionRequest,
    ProducerActionRequestError, ProducerActionResult, ProducerActionResultError,
    ProducerActionStatus, ProducerCapabilityStatus, ProducerFailureEvidence,
    ProducerFingerprintScopes, ProducerLaunchSnapshot, ProducerRetryGuidance, SchemaSummary,
    SchemaSummaryBuildError, SchemaSummaryEvidence,
};
use recite_core::{
    ContentFingerprint, ProducerFingerprint, ProducerIdentity, ProjectSchema, SchemaFingerprint,
    SpeakerDefinition, load_schema_manifest_str,
};

const GENERATED: &str = include_str!("../../../fixtures/schema/valid/full_manifest.json");

fn generated_schema() -> ProjectSchema {
    load_schema_manifest_str("full_manifest.json", GENERATED)
        .schema
        .expect("generated fixture lowers")
}

fn launch() -> ProducerLaunchSnapshot {
    ProducerLaunchSnapshot::from_schema(&generated_schema()).expect("producer launch evidence")
}

fn output() -> ProducerActionEvidence {
    ProducerActionEvidence::from_schema(&generated_schema()).expect("producer output evidence")
}

fn producer() -> ProducerIdentity {
    launch().producer().clone()
}

fn request() -> ProducerActionRequest {
    ProducerActionRequest::regenerate(output(), launch()).expect("request")
}

fn failure() -> ProducerFailureEvidence {
    ProducerFailureEvidence::new(
        producer(),
        "producer-input-invalid",
        Some("input requires correction".to_owned()),
    )
    .expect("failure")
    .with_retry_guidance(ProducerRetryGuidance::RetryAfterCorrection)
}

fn changed_scopes() -> ProducerFingerprintScopes {
    let snapshot = launch();
    let current = snapshot.input_fingerprints();
    let changed_manifest = vec![ProducerFingerprint {
        id: "content/changed".to_owned(),
        kind: "directory".to_owned(),
        algorithm: "blake3".to_owned(),
        value: "different".to_owned(),
    }];
    ProducerFingerprintScopes::new(
        changed_manifest,
        current
            .registries()
            .iter()
            .map(|(name, values)| (name.clone(), values.clone())),
        current
            .metadata_domains()
            .iter()
            .map(|(name, values)| (name.clone(), values.clone())),
    )
}

#[test]
fn request_identity_is_deterministic_and_covers_every_launch_scope() {
    let first = request();
    let second = request();
    assert_eq!(first.identity(), second.identity());
    assert!(
        !first
            .launch_snapshot()
            .input_fingerprints()
            .registries()
            .is_empty()
    );
    assert!(
        !first
            .launch_snapshot()
            .input_fingerprints()
            .metadata_domains()
            .is_empty()
    );

    let omitted = ProducerLaunchSnapshot::new(
        producer(),
        ProducerFingerprintScopes::new(
            launch().input_fingerprints().manifest().iter().cloned(),
            [],
            [],
        ),
    );
    let omitted_expected = ProducerActionEvidence::new(
        producer(),
        output().schema_fingerprint().clone(),
        output().content_fingerprint().clone(),
        omitted.input_fingerprints().clone(),
        output().output_fingerprint().cloned(),
    )
    .expect("omitted evidence");
    let omitted_request =
        ProducerActionRequest::regenerate(omitted_expected, omitted).expect("request");
    assert_ne!(first.identity(), omitted_request.identity());
}

#[test]
fn changed_inputs_are_stale_but_changed_outputs_are_valid_success() {
    let request = request();
    let observed = ProducerLaunchSnapshot::new(producer(), changed_scopes());
    let stale = ProducerActionResult::stale(&request, observed).expect("stale");
    assert_eq!(stale.status(), ProducerActionStatus::Stale);
    assert!(stale.observed_stale_snapshot().is_some());

    let mut reloaded_schema = generated_schema();
    reloaded_schema.speakers.insert(
        "new_speaker".to_owned(),
        SpeakerDefinition {
            display_name: Some("New speaker".to_owned()),
        },
    );
    let reloaded = ProducerActionEvidence::from_schema(&reloaded_schema).expect("reloaded output");
    assert_eq!(
        reloaded.input_fingerprints(),
        request.launch_snapshot().input_fingerprints()
    );
    let success = ProducerActionResult::succeeded(&request, reloaded).expect("success");
    assert_eq!(success.status(), ProducerActionStatus::Succeeded);
    assert!(success.observed_stale_snapshot().is_none());

    assert!(matches!(
        ProducerActionResult::stale(&request, request.launch_snapshot().clone()),
        Err(ProducerActionResultError::StaleWithoutChangedInputs)
    ));
}

#[test]
fn retry_requires_the_exact_failed_result() {
    let request = request();
    let failed = ProducerActionResult::failed(&request, failure()).expect("failed result");
    let retry = failed.retry_request().expect("retry");
    assert!(matches!(
        retry.operation(),
        ProducerActionOperation::Retry { originating_request, .. }
            if originating_request == failed.request_identity()
    ));
    assert_eq!(retry.launch_snapshot(), request.launch_snapshot());

    let fabricated = ProducerActionOperation::Retry {
        failure: failure(),
        originating_request: request.identity().clone(),
    };
    assert!(matches!(
        ProducerActionRequest::new(producer(), fabricated, output(), launch()),
        Err(ProducerActionRequestError::RetryRequiresFailedResult)
    ));
    assert!(matches!(
        ProducerActionResult::cancelled(&request).retry_request(),
        Err(ProducerActionRequestError::NotFailedResult)
    ));
}

#[test]
fn output_evidence_rejects_no_schema_and_inconsistent_content() {
    let valid = output();
    let no_schema = ProducerActionEvidence::new(
        producer(),
        SchemaFingerprint::NoSchema,
        valid.content_fingerprint().clone(),
        valid.input_fingerprints().clone(),
        None,
    );
    assert!(matches!(
        no_schema,
        Err(recite_compiler::ProducerActionEvidenceError::NoSchemaFingerprint)
    ));
    let inconsistent = ProducerActionEvidence::new(
        producer(),
        valid.schema_fingerprint().clone(),
        ContentFingerprint::blake3(vec![8; 32]).expect("digest"),
        valid.input_fingerprints().clone(),
        None,
    );
    assert!(matches!(
        inconsistent,
        Err(recite_compiler::ProducerActionEvidenceError::SchemaContentMismatch)
    ));
}

#[test]
fn failure_guidance_is_structured_and_only_bound_failures_project_retry() {
    let request = request();
    let failed_result = ProducerActionResult::failed(&request, failure()).expect("failed");
    let bare_evidence = SchemaSummaryEvidence::builder(producer())
        .capability(ProducerCapabilityStatus::Supported)
        .current_failure(failure())
        .build()
        .expect("evidence");
    let bare_summary =
        SchemaSummary::from_schema_with_evidence(&generated_schema(), Some(&bare_evidence))
            .expect("summary");
    assert_eq!(bare_summary.capability().producer_actions().len(), 1);

    let evidence = SchemaSummaryEvidence::builder(producer())
        .capability(ProducerCapabilityStatus::Supported)
        .current_failure(failure())
        .failed_result(failed_result.clone())
        .build()
        .expect("bound evidence");
    let summary = SchemaSummary::from_schema_with_evidence(&generated_schema(), Some(&evidence))
        .expect("summary");
    assert_eq!(summary.capability().producer_actions().len(), 2);
    assert!(matches!(
        summary.capability().producer_actions()[1].request().operation(),
        ProducerActionOperation::Retry { originating_request, .. }
            if originating_request == failed_result.request_identity()
    ));
}

#[test]
fn stale_failed_results_cannot_be_attached_to_a_current_summary() {
    let changed_launch = ProducerLaunchSnapshot::new(producer(), changed_scopes());
    let changed_expected = ProducerActionEvidence::new(
        producer(),
        output().schema_fingerprint().clone(),
        output().content_fingerprint().clone(),
        changed_launch.input_fingerprints().clone(),
        output().output_fingerprint().cloned(),
    )
    .expect("changed expected evidence");
    let changed_request =
        ProducerActionRequest::regenerate(changed_expected, changed_launch).expect("request");
    let stale_failure =
        ProducerActionResult::failed(&changed_request, failure()).expect("failed result");
    let evidence = SchemaSummaryEvidence::builder(producer())
        .capability(ProducerCapabilityStatus::Supported)
        .current_failure(failure())
        .failed_result(stale_failure)
        .build()
        .expect("evidence builder remains structural");
    assert!(matches!(
        SchemaSummary::from_schema_with_evidence(&generated_schema(), Some(&evidence)),
        Err(SchemaSummaryBuildError::FailedResultSnapshotMismatch)
    ));
}
