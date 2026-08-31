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

#[cfg(unix)]
const SIMPLE_MARKER: &str = "u1~6d61726b6572";
#[cfg(windows)]
const SIMPLE_MARKER: &str = "w1~006d00610072006b00650072";
#[cfg(not(any(unix, windows)))]
const SIMPLE_MARKER: &str = "p1~006d00610072006b00650072";

#[cfg(unix)]
const FIRST_STAGE_MARKER: &str = "u1~73746167652f66697273740a2e746d70";
#[cfg(windows)]
const FIRST_STAGE_MARKER: &str =
    "w1~00730074006100670065002f00660069007200730074000a002e0074006d0070";
#[cfg(not(any(unix, windows)))]
const FIRST_STAGE_MARKER: &str =
    "p1~00730074006100670065002f00660069007200730074000a002e0074006d0070";

#[cfg(unix)]
const SECOND_STAGE_MARKER: &str = "u1~73746167652f7365636f6e642e746d70";
#[cfg(windows)]
const SECOND_STAGE_MARKER: &str =
    "w1~00730074006100670065002f007300650063006f006e0064002e0074006d0070";
#[cfg(not(any(unix, windows)))]
const SECOND_STAGE_MARKER: &str =
    "p1~00730074006100670065002f007300650063006f006e0064002e0074006d0070";

#[test]
fn coordinator_and_freshness_errors_retain_recovery_for_host() {
    let messages = Messages::load(&crate::i18n::UiLocale::default()).expect("messages");
    let record = ProjectBuildRecovery::new(
        PathBuf::from("marker"),
        super::super::recovery::ProjectBuildRecoveryReason::StageCleanupFailed,
    );
    let mut stderr = Vec::new();
    crate::watch::report_build_result(
        &mut stderr,
        Err(crate::error::CliError::WatchCoordinator {
            source: recite_compiler::BuildRunError::MissingAuthority,
            recovery: vec![record.clone()],
        }),
        &messages,
    )
    .expect("coordinator report");
    let output = String::from_utf8(stderr).expect("stderr");
    assert!(output.contains("coordinator has no publication authority"));
    assert!(output.contains("recovery markers"));

    let mut stderr = Vec::new();
    crate::watch::report_build_result(
        &mut stderr,
        Err(crate::error::CliError::WatchRecovery {
            source: Box::new(crate::error::CliError::Read {
                path: PathBuf::from("schema.json"),
                source: io::Error::new(io::ErrorKind::NotFound, "schema missing"),
            }),
            recovery: vec![record],
        }),
        &messages,
    )
    .expect("freshness report");
    let output = String::from_utf8(stderr).expect("stderr");
    assert!(output.contains("failed to read"));
    assert!(output.contains("recovery markers"));
}

#[test]
fn freshness_recovery_localizes_nested_error_and_wrapper() {
    let mut resource = recite_ui::DEFAULT_RESOURCE.to_owned();
    for (id, replacement) in [
        ("watch-build-failed", "alt-failed {$error}"),
        ("cli-error-read", "alt-read {$path} {$source}"),
        ("watch-build-recovery-notice", "alt-notice {$records}"),
        (
            "watch-build-recovery-record",
            "alt-record {$marker} {$reason}{$detail}",
        ),
        ("watch-build-recovery-reason-stage-cleanup", "alt-stage"),
    ] {
        let mut found = false;
        let mut lines = Vec::new();
        for line in resource.lines() {
            if line.starts_with(&format!("{id} = ")) {
                found = true;
                lines.push(format!("{id} = {replacement}"));
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
    let mut stderr = Vec::new();
    crate::watch::report_build_result(
        &mut stderr,
        Err(crate::error::CliError::WatchRecovery {
            source: Box::new(crate::error::CliError::Read {
                path: PathBuf::from("schema.json"),
                source: io::Error::new(io::ErrorKind::NotFound, "schema missing"),
            }),
            recovery: vec![ProjectBuildRecovery::new(
                PathBuf::from("marker"),
                super::super::recovery::ProjectBuildRecoveryReason::StageCleanupFailed,
            )],
        }),
        &messages,
    )
    .expect("localized freshness report");
    assert_eq!(
        String::from_utf8(stderr).expect("stderr"),
        format!(
            "alt-failed alt-read schema.json schema missing\nalt-notice ; recovery markers: alt-record {SIMPLE_MARKER} alt-stage\n"
        )
    );
}

#[test]
fn recovery_deduplication_uses_marker_and_reason_not_io_detail() {
    let messages = Messages::load(&crate::i18n::UiLocale::default()).expect("messages");
    let marker = PathBuf::from("marker");
    let first = ProjectBuildRecovery::new(
        marker.clone(),
        super::super::recovery::ProjectBuildRecoveryReason::StageCleanupFailed,
    );
    let second = ProjectBuildRecovery::with_io(
        marker,
        super::super::recovery::ProjectBuildRecoveryReason::StageCleanupFailed,
        &io::Error::new(io::ErrorKind::PermissionDenied, "denied"),
    );
    let output = super::super::build::format_recovery_required(&messages, 1, &[first, second]);
    assert!(output.contains(&format!("recovery markers: {SIMPLE_MARKER}")));
    assert_eq!(output.matches(SIMPLE_MARKER).count(), 1);
}

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
            "alt-record marker={$marker} reason={$reason} detail={$detail}",
        ),
        ("watch-build-recovery-reason-stage-cleanup", "alt-stage"),
        (
            "watch-build-recovery-reason-publication-uncommitted",
            "alt-uncommitted",
        ),
        (
            "watch-build-recovery-detail-io",
            "alt-io {$kind} {$raw_os_error} {$message}",
        ),
        (
            "watch-build-recovery-io-permission-denied",
            "alt-permission",
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
    let first = ProjectBuildRecovery::with_io(
        PathBuf::from("stage/first\n.tmp"),
        super::super::recovery::ProjectBuildRecoveryReason::StageCleanupFailed,
        &io::Error::new(io::ErrorKind::PermissionDenied, "denied"),
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
        format!(
            "alt-required count=2 records=; recovery markers: alt-record marker={FIRST_STAGE_MARKER} reason=alt-stage detail=alt-io alt-permission  denied\nalt-record marker={SECOND_STAGE_MARKER} reason=alt-uncommitted detail="
        )
    );
}
