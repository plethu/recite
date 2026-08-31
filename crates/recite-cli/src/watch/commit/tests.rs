use std::fs;
use std::io;

use recite_compiler::{
    BuildCandidate, BuildGeneration, BuildInput, BuildRequest, BuildTarget,
    PreparedPublishIdentity, PublishOutcome, SnapshotGeneration,
};
use recite_core::DocumentKey;
use tempfile::TempDir;

use super::super::publisher::{ProjectPreparedBuild, StagedTarget};
use super::super::staging::{self, StagedOutput};
use super::{ProjectBuildRecovery, commit_prepared_with};

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
                output,
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
}
