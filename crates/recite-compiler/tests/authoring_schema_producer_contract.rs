#![cfg(test)]

use recite_compiler::{
    ProducerActionEvidence, ProducerActionOperation, ProducerActionRequest,
    ProducerActionRequestError, ProducerActionResult, ProducerActionStatus,
    ProducerCapabilityStatus, ProducerFailureEvidence, ProducerFingerprintScopes,
    ProducerLaunchSnapshot, ProducerRetryGuidance, SchemaAction, SchemaSummary,
    SchemaSummaryEvidence,
};
use recite_core::{
    ProducerFingerprint, ProducerIdentity, ProjectSchema, SpeakerDefinition,
    load_schema_manifest_str,
};

const GENERATED: &str = include_str!("../../../fixtures/schema/valid/full_manifest.json");

fn generated_schema() -> ProjectSchema {
    load_schema_manifest_str("full_manifest.json", GENERATED)
        .schema
        .expect("generated fixture lowers")
}

fn launch(schema: &ProjectSchema) -> ProducerLaunchSnapshot {
    ProducerLaunchSnapshot::from_schema(schema).expect("producer launch evidence")
}

fn output(schema: &ProjectSchema) -> ProducerActionEvidence {
    ProducerActionEvidence::from_schema(schema).expect("producer output evidence")
}

fn producer(schema: &ProjectSchema) -> ProducerIdentity {
    launch(schema).producer().clone()
}

fn request(schema: &ProjectSchema) -> ProducerActionRequest {
    ProducerActionRequest::regenerate(output(schema), launch(schema)).expect("request")
}

fn failure(schema: &ProjectSchema) -> ProducerFailureEvidence {
    ProducerFailureEvidence::new(
        producer(schema),
        "producer-input-invalid",
        Some("input requires correction".to_owned()),
    )
    .expect("failure")
    .with_retry_guidance(ProducerRetryGuidance::RetryAfterCorrection)
}

fn changed_schema() -> ProjectSchema {
    let mut schema = generated_schema();
    schema.speakers.insert(
        "new_speaker".to_owned(),
        SpeakerDefinition {
            display_name: Some("New speaker".to_owned()),
        },
    );
    schema
        .registries
        .get_mut("item")
        .expect("item registry")
        .producer_fingerprints[0]
        .value = "6f1e".to_owned();
    schema
}

#[test]
fn request_identity_is_deterministic_and_covers_every_launch_scope() {
    let schema = generated_schema();
    let first = request(&schema);
    let second = request(&schema);
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

    let launch_without_named_scopes = ProducerLaunchSnapshot::new(
        producer(&schema),
        ProducerFingerprintScopes::new(
            launch(&schema)
                .input_fingerprints()
                .manifest()
                .iter()
                .cloned(),
            [],
            [],
        )
        .expect("scope snapshot"),
    );
    let omitted_request =
        ProducerActionRequest::regenerate(output(&schema), launch_without_named_scopes)
            .expect("request");
    assert_ne!(first.identity(), omitted_request.identity());
}

#[test]
fn previous_output_and_current_launch_are_independent_and_success_uses_current_output() {
    let previous = generated_schema();
    let current = changed_schema();
    let request = ProducerActionRequest::regenerate(output(&previous), launch(&current))
        .expect("regeneration request accepts changed launch inputs");
    assert_ne!(
        request.expected().input_fingerprints(),
        request.launch_snapshot().input_fingerprints()
    );

    let success = ProducerActionResult::succeeded(&request, &current).expect("success");
    assert_eq!(success.status(), ProducerActionStatus::Succeeded);
    assert_eq!(
        success
            .evidence()
            .expect("success evidence")
            .input_fingerprints(),
        request.launch_snapshot().input_fingerprints()
    );
    assert!(success.observed_stale_snapshot().is_none());

    let stale = ProducerActionResult::stale(&request, launch(&previous)).expect("stale");
    assert_eq!(stale.status(), ProducerActionStatus::Stale);
    assert!(stale.observed_stale_snapshot().is_some());
    assert!(matches!(
        ProducerActionResult::stale(&request, request.launch_snapshot().clone()),
        Err(recite_compiler::ProducerActionResultError::StaleWithoutChangedInputs)
    ));
}

#[test]
fn corrected_retry_keeps_old_failure_identity_and_uses_new_launch() {
    let previous = generated_schema();
    let current = changed_schema();
    let original = request(&previous);
    let failed =
        ProducerActionResult::failed(&original, failure(&previous)).expect("failed result");
    let retry = failed
        .retry_request_with_launch(launch(&current))
        .expect("corrected retry");

    assert_eq!(retry.launch_snapshot(), &launch(&current));
    assert_eq!(retry.expected(), original.expected());
    assert!(matches!(
        retry.operation(),
        ProducerActionOperation::Retry { originating_request, .. }
            if originating_request == failed.request_identity()
    ));
    assert!(matches!(
        failed.retry_request(),
        Err(ProducerActionRequestError::RetryRequiresCurrentLaunch)
    ));
}

#[test]
fn retry_requires_the_exact_failed_result() {
    let schema = generated_schema();
    let request = request(&schema);
    let fabricated = ProducerActionOperation::Retry {
        failure: failure(&schema),
        originating_request: request.identity().clone(),
    };
    assert!(matches!(
        ProducerActionRequest::new(
            producer(&schema),
            fabricated,
            output(&schema),
            launch(&schema)
        ),
        Err(ProducerActionRequestError::RetryRequiresFailedResult)
    ));
    assert!(matches!(
        ProducerActionResult::cancelled(&request).retry_request(),
        Err(ProducerActionRequestError::NotFailedResult)
    ));
    let retry_now_failure = failure(&schema).with_retry_guidance(ProducerRetryGuidance::RetryNow);
    let retry_now = ProducerActionResult::failed(&request, retry_now_failure)
        .expect("retry-now result")
        .retry_request()
        .expect("retry from exact result");
    assert_eq!(
        retry_now.operation(),
        &ProducerActionOperation::Retry {
            failure: failure(&schema).with_retry_guidance(ProducerRetryGuidance::RetryNow),
            originating_request: request.identity().clone(),
        }
    );
}

#[test]
fn success_only_derives_evidence_from_reloaded_schema() {
    let changed = changed_schema();
    let request = ProducerActionRequest::regenerate(output(&generated_schema()), launch(&changed))
        .expect("request");
    let result = ProducerActionResult::succeeded(&request, &changed).expect("reloaded output");
    assert_eq!(
        result.evidence().expect("evidence").schema_fingerprint(),
        &changed.canonical_fingerprint()
    );
}

#[test]
fn bare_current_failure_has_no_usable_retry_action() {
    let schema = generated_schema();
    let producer = producer(&schema);
    let evidence = SchemaSummaryEvidence::builder(producer.clone())
        .capability(ProducerCapabilityStatus::Supported)
        .current_failure(failure(&schema))
        .build()
        .expect("evidence");
    let summary =
        SchemaSummary::from_schema_with_evidence(&schema, Some(&evidence)).expect("summary");
    assert!(
        summary
            .capability()
            .actions()
            .iter()
            .all(|action| !matches!(action, SchemaAction::RetryProducerFailure { .. }))
    );
    assert_eq!(summary.capability().producer_actions().len(), 1);
}

#[test]
fn exact_failed_result_projects_retry_on_current_summary() {
    let previous = generated_schema();
    let current = changed_schema();
    let original = request(&previous);
    let failed =
        ProducerActionResult::failed(&original, failure(&previous)).expect("failed result");
    let evidence = SchemaSummaryEvidence::builder(producer(&current))
        .capability(ProducerCapabilityStatus::Supported)
        .failed_result(failed.clone())
        .build()
        .expect("evidence");
    let summary = SchemaSummary::from_schema_with_evidence(&current, Some(&evidence))
        .expect("summary accepts old failure and current launch");
    assert!(
        summary
            .capability()
            .supports(&SchemaAction::RetryProducerFailure {
                producer: producer(&current),
            })
    );
    assert!(matches!(
        summary.capability().producer_actions()[1].request().operation(),
        ProducerActionOperation::Retry { originating_request, .. }
            if originating_request == failed.request_identity()
    ));
    assert_eq!(
        summary.capability().producer_actions()[1]
            .request()
            .launch_snapshot(),
        &launch(&current)
    );
}

#[test]
fn duplicate_scopes_and_keys_are_rejected_in_any_order() {
    let fingerprint = |id: &str, value: &str| ProducerFingerprint {
        id: id.to_owned(),
        kind: "file".to_owned(),
        algorithm: "blake3".to_owned(),
        value: value.to_owned(),
    };
    assert!(
        ProducerFingerprintScopes::new(
            [fingerprint("same", "b"), fingerprint("same", "a")],
            [],
            [],
        )
        .is_err()
    );
    assert!(
        ProducerFingerprintScopes::new(
            [],
            [
                ("items".to_owned(), Vec::new()),
                ("items".to_owned(), Vec::new())
            ],
            [],
        )
        .is_err()
    );
    assert!(
        ProducerFingerprintScopes::new(
            [],
            [(
                "items".to_owned(),
                vec![fingerprint("same", "b"), fingerprint("same", "a")],
            )],
            [],
        )
        .is_err()
    );
    assert!(
        ProducerFingerprintScopes::new(
            [],
            [],
            [(
                "tone".to_owned(),
                vec![fingerprint("same", "b"), fingerprint("same", "a")],
            )],
        )
        .is_err()
    );
}
