use std::fs;
use std::io;
use std::path::PathBuf;

use recite_compiler::{
    BuildCandidate, BuildGeneration, BuildInput, BuildRequest, BuildTarget,
    PreparedPublishIdentity, PublishOutcome, SnapshotGeneration,
};
use recite_core::DocumentKey;
use tempfile::TempDir;

use super::super::publisher::{ProjectPreparedBuild, StagedTarget};
use super::super::staging::{self, StagedOutput};
use super::{ProjectBuildRecovery, commit_prepared_with};
use crate::i18n::Messages;

#[test]
fn post_rename_error_is_indeterminate_with_visible_new_bytes() {
    let temp = TempDir::new().unwrap_or_else(|error| panic!("temporary directory: {error}"));
    let output = temp.path().join("dialogue.recitec");
    let stage = temp.path().join("dialogue.recitec.stage");
    fs::write(&output, b"old").unwrap_or_else(|error| panic!("old output: {error}"));
    fs::write(&stage, b"new").unwrap_or_else(|error| panic!("stage: {error}"));
    let target =
        BuildTarget::new("dialogue.recitec").unwrap_or_else(|error| panic!("target: {error}"));
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
    let prepared = ProjectPreparedBuild {
        identity: PreparedPublishIdentity::for_request(&request, vec![candidate]),
        staged: vec![StagedTarget {
            target,
            file: StagedOutput {
                temp: stage,
                output: output.clone(),
            },
        }],
    };
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
        fs::read(temp.path().join("dialogue.recitec"))
            .unwrap_or_else(|error| panic!("published bytes: {error}")),
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
    let output = temp.path().join("dialogue.recitec");
    let stage = temp.path().join("dialogue.recitec.stage");
    fs::write(&output, b"old").expect("old output");
    fs::write(&stage, b"new").expect("stage");
    let target = BuildTarget::new("dialogue.recitec").expect("target");
    let request = BuildRequest::new(
        BuildGeneration::new(1),
        SnapshotGeneration::new(1),
        [BuildInput::saved_source(
            DocumentKey::new("dialogue/main.recite").expect("key"),
            "source",
        )],
    )
    .expect("request");
    let candidate = BuildCandidate::new(target.clone(), b"new".to_vec());
    let prepared = ProjectPreparedBuild {
        identity: PreparedPublishIdentity::for_request(&request, vec![candidate]),
        staged: vec![StagedTarget {
            target,
            file: StagedOutput {
                temp: stage,
                output: output.clone(),
            },
        }],
    };
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
fn recovery_records_use_alternate_typed_fluent_contract() {
    let mut resource = recite_ui::DEFAULT_RESOURCE.to_owned();
    for (id, replacement) in [
        (
            "watch-build-recovery-required",
            "alt-required count={$count} records={$records}",
        ),
        (
            "watch-build-recovery-record",
            "alt-record marker={$marker} reason={$reason}",
        ),
        ("watch-build-recovery-reason-stage-cleanup", "alt-stage"),
        (
            "watch-build-recovery-reason-publication-uncommitted",
            "alt-uncommitted",
        ),
    ] {
        let mut found = false;
        let mut lines = Vec::new();
        for line in resource.lines() {
            if line.starts_with(&format!("{id} = ")) {
                found = true;
                let mut replacement_lines = replacement.lines();
                if let Some(first) = replacement_lines.next() {
                    lines.push(format!("{id} = {first}"));
                    lines.extend(replacement_lines.map(str::to_owned));
                }
            } else {
                lines.push(line.to_owned());
            }
        }
        resource = lines.join("\n");
        resource.push('\n');
        assert!(found, "missing resource {id}");
    }
    recite_ui::UiContract::default()
        .validate(&resource)
        .expect("alternate resource contract");
    let messages = Messages::from_resources(
        "en-GB".parse().expect("locale"),
        [
            (
                "en-US".parse().expect("locale"),
                recite_ui::DEFAULT_RESOURCE.to_owned(),
            ),
            ("en-GB".parse().expect("locale"), resource),
        ],
    )
    .expect("alternate messages");
    let first = ProjectBuildRecovery::new(
        PathBuf::from("stage/first\n.tmp"),
        super::super::recovery::ProjectBuildRecoveryReason::StageCleanupFailed,
    );
    let second = ProjectBuildRecovery::new(
        PathBuf::from("stage/second.tmp"),
        super::super::recovery::ProjectBuildRecoveryReason::PublicationUncommitted,
    );
    assert_eq!(
        super::super::build::format_recovery_required(
            &messages,
            2,
            &[second.clone(), first.clone(), first],
        ),
        "alt-required count=2 records=; recovery markers: alt-record marker=stage/first\\n.tmp reason=alt-stage\nalt-record marker=stage/second.tmp reason=alt-uncommitted"
    );
}
