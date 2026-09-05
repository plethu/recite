use std::fs;

use recite_compiler::{
    BuildCandidate, BuildGeneration, BuildInput, BuildRequest, BuildTarget, FreshnessAssessment,
    PublishOutcome, RecoveryNeeded, SnapshotGeneration, StaleReason,
};
use serde_json::Value;
use tempfile::TempDir;

use super::{artifact_metadata_for_publication, failure, freshness, publication, recovery_record};
use crate::structured::errors::{ErrorCategory, ErrorCode, StructuredError};
use crate::watch::{ProjectBuildRecovery, ProjectBuildRecoveryReason};

fn target(value: &str) -> BuildTarget {
    BuildTarget::new(value.to_owned()).expect("target")
}

#[test]
fn partial_publication_maps_committed_artifacts_and_recovery() {
    let temp = TempDir::new().expect("tempdir");
    fs::create_dir_all(temp.path().join("compiled")).expect("output directory");
    fs::write(temp.path().join("compiled/a.recitec"), b"a").expect("committed output");
    let committed = target("compiled/a.recitec");
    let failed = target("compiled/b.recitec");
    let remaining = target("compiled/c.recitec");
    let outcome = PublishOutcome::Partial {
        committed: vec![committed.clone()],
        failed: failed.clone(),
        remaining: vec![remaining.clone()],
        recovery: RecoveryNeeded::for_targets(vec![failed.clone(), committed.clone()]),
    };

    let publication = serde_json::to_value(publication(&outcome)).expect("publication JSON");
    assert_eq!(
        publication,
        serde_json::json!({
            "type": "partial",
            "committed": ["compiled/a.recitec"],
            "failed": "compiled/b.recitec",
            "remaining": ["compiled/c.recitec"],
            "recovery": ["compiled/a.recitec", "compiled/b.recitec"]
        })
    );
    let candidate = BuildCandidate::new(committed.clone(), b"candidate-bytes".to_vec());
    let artifacts = artifact_metadata_for_publication(temp.path(), Some(&outcome), &[candidate])
        .expect("committed artifact metadata");
    assert_eq!(artifacts.len(), 1);
    assert_eq!(artifacts[0].size_bytes, 15);
    assert_eq!(
        artifacts[0].path,
        crate::schema_inspection::MachinePathProjection::Utf8(
            temp.path()
                .join("compiled/a.recitec")
                .to_string_lossy()
                .into_owned()
        )
    );
}

#[test]
fn indeterminate_publication_maps_attempts_without_artifact_claims() {
    let temp = TempDir::new().expect("tempdir");
    fs::create_dir_all(temp.path().join("compiled")).expect("output directory");
    fs::write(temp.path().join("compiled/a.recitec"), b"a").expect("attempted output");
    let outcome = PublishOutcome::Indeterminate {
        attempted: vec![target("compiled/a.recitec")],
        recovery: RecoveryNeeded::for_targets(vec![target("compiled/a.recitec")]),
    };

    let publication = serde_json::to_value(publication(&outcome)).expect("publication JSON");
    assert_eq!(
        publication,
        serde_json::json!({
            "type": "indeterminate",
            "attempted": ["compiled/a.recitec"],
            "recovery": ["compiled/a.recitec"]
        })
    );
    assert!(
        artifact_metadata_for_publication(temp.path(), Some(&outcome), &[])
            .expect("indeterminate metadata")
            .is_empty()
    );
}

#[test]
fn stale_freshness_and_recovery_use_tagged_machine_dtos() {
    let request = BuildRequest::new(
        BuildGeneration::new(1),
        SnapshotGeneration::new(1),
        [BuildInput::saved_source(
            recite_core::DocumentKey::new("dialogue/main.recite").expect("input key"),
            "source",
        )],
    )
    .expect("request");
    let assessment = FreshnessAssessment::stale(
        request.fingerprints().clone(),
        vec![StaleReason::Fingerprints, StaleReason::BuildGeneration],
    );
    let value = serde_json::to_value(freshness(&assessment)).expect("freshness JSON");
    assert_eq!(value["type"], "stale");
    assert_eq!(
        value["reasons"],
        serde_json::json!([
            {"type": "build_generation"},
            {"type": "fingerprints"}
        ])
    );

    let recovery = ProjectBuildRecovery::new(
        std::path::PathBuf::from("compiled/.stage"),
        ProjectBuildRecoveryReason::PublicationIndeterminate,
    );
    let value = serde_json::to_value(recovery_record(&recovery)).expect("recovery JSON");
    assert_eq!(value["reason"], "publication_indeterminate");
    assert_eq!(value["marker"]["encoding"], "utf8");
}

#[test]
fn failure_mapping_is_typed_without_host_prose() {
    let value = failure(&recite_compiler::BuildResultFailure::Freshness {
        reason: recite_compiler::FreshnessFailureReason::RecheckFailed,
    });
    let value: Value = serde_json::to_value(value).expect("failure JSON");
    assert_eq!(value["type"], "freshness");

    let value = failure(&recite_compiler::BuildResultFailure::Engine {
        reason: recite_compiler::BuildFailureReason::Host,
    });
    let value: Value = serde_json::to_value(value).expect("engine failure JSON");
    assert_eq!(
        value,
        serde_json::json!({"type": "engine", "reason": "host"})
    );
}

#[test]
fn preparation_diagnostics_do_not_claim_a_build_publication_attempt() {
    let value =
        super::BuildCompletedData::from_diagnostics(0, &["recite.project.toml".into()], &[])
            .expect("diagnostic completion");
    let value: Value = serde_json::to_value(value).expect("completion JSON");
    assert_eq!(value["outcome"]["type"], "diagnostics");
    assert_eq!(
        value["publication"],
        serde_json::json!({"type":"not_attempted","reason":"preparation_failed"})
    );
}

#[test]
fn preparation_inputs_are_sorted_and_deduplicated() {
    let error = StructuredError {
        category: ErrorCategory::Io,
        code: ErrorCode::Read,
        operation: "read",
        path: None,
        related_path: None,
        details: None,
    };
    let value = super::BuildCompletedData::from_error(
        0,
        &["z-input".into(), "a-input".into(), "z-input".into()],
        error,
    );
    let value: Value = serde_json::to_value(value).expect("completion JSON");
    assert_eq!(value["inputs"], serde_json::json!(["a-input", "z-input"]));
}
