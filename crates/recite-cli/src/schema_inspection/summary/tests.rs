use recite_compiler::{
    ProducerActionEvidence, ProducerActionRequest, ProducerActionResult, ProducerCapabilityStatus,
    ProducerFailureEvidence, ProducerLaunchSnapshot, ProducerRetryGuidance, SchemaSummary,
    SchemaSummaryEvidence,
};
use recite_core::load_schema_manifest_str;

use super::from_summary;
use crate::schema_inspection::path::machine_path;
use crate::schema_inspection::{input::InputFormat, model::SchemaInspectionProjection};

const GENERATED: &str = include_str!("../../../../../fixtures/schema/valid/full_manifest.json");

#[test]
fn retry_failure_is_projected_as_structured_producer_action() {
    let previous = load_schema_manifest_str("previous.json", GENERATED)
        .schema
        .expect("generated fixture lowers");
    let mut current = previous.clone();
    current
        .registries
        .get_mut("item")
        .expect("item registry")
        .producer_fingerprints[0]
        .value = "changed".to_owned();

    let producer = ProducerLaunchSnapshot::from_schema(&previous)
        .expect("previous launch")
        .producer()
        .clone();
    let original = ProducerActionRequest::regenerate(
        ProducerActionEvidence::from_schema(&previous).expect("previous output"),
        ProducerLaunchSnapshot::from_schema(&previous).expect("previous launch"),
    )
    .expect("regeneration request");
    let failure = ProducerFailureEvidence::new(
        producer.clone(),
        "producer-input-invalid",
        Some("input requires correction".to_owned()),
    )
    .expect("failure")
    .with_retry_guidance(ProducerRetryGuidance::RetryAfterCorrection);
    let failed = ProducerActionResult::failed(&original, failure).expect("failed result");
    let evidence = SchemaSummaryEvidence::builder(producer)
        .capability(ProducerCapabilityStatus::Supported)
        .failed_result(failed)
        .build()
        .expect("summary evidence");
    let summary =
        SchemaSummary::from_schema_with_evidence(&current, Some(&evidence)).expect("summary");
    let projection: SchemaInspectionProjection = from_summary(
        &summary,
        &current,
        InputFormat::GeneratedJson,
        machine_path(std::path::Path::new("generated.json")),
    )
    .expect("projection");
    let json = serde_json::to_value(projection).expect("projection JSON");
    assert_eq!(
        json["capability"]["actions"],
        serde_json::json!(["invoke_producer", "retry_producer_failure"])
    );
    assert_eq!(
        json["capability"]["producer_actions"]
            .as_array()
            .map(Vec::len),
        Some(2)
    );
    assert_eq!(
        json["capability"]["producer_actions"][0]["operation"]["kind"],
        "regenerate"
    );
    assert_eq!(
        json["capability"]["producer_actions"][1]["operation"]["kind"],
        "retry"
    );
    assert_eq!(
        json["capability"]["producer_actions"],
        json["types"][0]["capability"]["producer_actions"]
    );
    assert_eq!(
        json["types"][0]["capability"]["producer_actions"][1]["operation"]["kind"],
        "retry"
    );
    assert_eq!(
        json["types"][0]["capability"]["producer_actions"][1]["operation"]["failure"]["retry_guidance"],
        "retry_after_correction"
    );
}
