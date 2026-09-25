use std::cell::Cell;
use std::fs;
use std::io;
use std::path::PathBuf;

use recite_compiler::authoring::{
    BuildCandidate, BuildGeneration, BuildInput, BuildRequest, BuildTarget,
    PreparedPublishIdentity, PublishOutcome, SnapshotGeneration,
};
use recite_core::DocumentKey;
use tempfile::TempDir;

use super::super::publisher::{ProjectPreparedBuild, StagedTarget};
use super::super::staging::{self, StagedOutput};
use super::{ProjectBuildRecovery, commit_prepared_with};

fn prepared_build(
    temp: &TempDir,
    target_name: &str,
    stage_name: &str,
) -> (ProjectPreparedBuild, PathBuf, PathBuf) {
    let output = temp.path().join(target_name);
    let stage = temp.path().join(stage_name);
    if let Some(parent) = output.parent() {
        fs::create_dir_all(parent).unwrap_or_else(|error| panic!("output parent: {error}"));
    }
    fs::write(&output, b"old").unwrap_or_else(|error| panic!("old output: {error}"));
    fs::write(&stage, b"new").unwrap_or_else(|error| panic!("stage: {error}"));
    let target = BuildTarget::new(target_name).unwrap_or_else(|error| panic!("target: {error}"));
    let request = BuildRequest::new(
        BuildGeneration::new(1),
        SnapshotGeneration::new(1),
        [BuildInput::saved_source(
            DocumentKey::new("dialogue/main.recite").unwrap_or_else(|error| panic!("key: {error}")),
            "source",
        )],
    )
    .unwrap_or_else(|error| panic!("request: {error}"));
    let candidate = BuildCandidate::new(target.clone(), b"new".to_vec());
    (
        ProjectPreparedBuild {
            identity: PreparedPublishIdentity::for_request(&request, vec![candidate]),
            staged: vec![StagedTarget {
                target,
                file: StagedOutput {
                    temp: stage.clone(),
                    output: output.clone(),
                },
            }],
        },
        output,
        stage,
    )
}

#[test]
fn post_rename_error_is_indeterminate_with_visible_new_bytes() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("temporary directory: {error}"));
    let (prepared, output, _) = prepared_build(&temp, "dialogue.recitec", "dialogue.recitec.stage");
    let mut recovery = Vec::<ProjectBuildRecovery>::new();
    let outcome =
        commit_prepared_with(
            temp.path(),
            prepared,
            &mut recovery,
            |staged| match staging::replace(staged) {
                staging::ReplaceOutcome::Committed => {
                    staging::ReplaceOutcome::Indeterminate(io::Error::other("post-rename"))
                }
                other => other,
            },
        );
    assert!(matches!(outcome, PublishOutcome::Indeterminate { .. }));
    assert_eq!(
        fs::read(output).unwrap_or_else(|error| panic!("published bytes: {error}")),
        b"new"
    );
    assert_eq!(recovery.len(), 1);
    assert_eq!(
        recovery[0].reason(),
        super::super::recovery::ProjectBuildRecoveryReason::PublicationIndeterminate
    );
}

#[test]
fn committed_cleanup_error_publishes_and_requires_recovery() {
    let temp = TempDir::new().expect("tempdir");
    let (prepared, output, _) = prepared_build(&temp, "dialogue.recitec", "dialogue.recitec.stage");
    let mut recovery = Vec::new();
    let outcome =
        commit_prepared_with(
            temp.path(),
            prepared,
            &mut recovery,
            |staged| match staging::replace(staged) {
                staging::ReplaceOutcome::Committed => {
                    staging::ReplaceOutcome::CommittedWithCleanup(io::Error::other("cleanup"))
                }
                other => other,
            },
        );
    assert!(matches!(outcome, PublishOutcome::Published { .. }));
    assert_eq!(fs::read(output).expect("published bytes"), b"new");
    assert_eq!(
        recovery[0].reason(),
        super::super::recovery::ProjectBuildRecoveryReason::StageCleanupFailed
    );
}

#[test]
fn commit_rechecks_output_boundary_before_replacement() {
    let temp = TempDir::new().expect("tempdir");
    let target_name = "compiled/blocked/out.recitec";
    let (prepared, output, stage) = prepared_build(&temp, target_name, "stage.tmp");
    let blocked = temp.path().join("compiled/blocked");
    let moved = temp.path().join("compiled/blocked.saved");
    fs::rename(&blocked, &moved).expect("move staged parent");
    fs::write(&blocked, b"not a directory").expect("blocking file");
    let target = BuildTarget::new(target_name).expect("target");
    let invoked = Cell::new(false);
    let mut recovery = Vec::new();
    let outcome = commit_prepared_with(temp.path(), prepared, &mut recovery, |_| {
        invoked.set(true);
        staging::ReplaceOutcome::Committed
    });

    assert!(
        !invoked.get(),
        "replacement must not run after boundary failure"
    );
    assert!(matches!(
        outcome,
        PublishOutcome::Partial {
            committed,
            failed,
            remaining,
            recovery,
        } if committed.is_empty()
            && failed == target
            && remaining.is_empty()
            && recovery.targets() == std::slice::from_ref(&target)
    ));
    assert_eq!(recovery.len(), 1);
    assert_eq!(
        recovery[0].reason(),
        super::super::recovery::ProjectBuildRecoveryReason::PublicationUncommitted
    );
    assert_eq!(fs::read(&stage).expect("stage bytes"), b"new");
    assert!(!output.exists());
    assert_eq!(
        fs::read_dir(&moved).expect("moved staged parent").count(),
        1
    );
}
