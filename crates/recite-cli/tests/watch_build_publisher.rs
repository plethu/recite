use std::fmt::Display;
use std::fs;
use std::path::Path;

use recite_cli::watch::{
    ProjectBuildPreparation, ProjectBuildPublisher, ProjectBuildRequest, ProjectBuildTarget,
};
use recite_compiler::{
    BuildCandidate, BuildControl, BuildPreparedHandle, BuildPublisher, BuildTarget,
    PublishAbortReason, PublishOutcome,
};
use tempfile::TempDir;

fn require<T, E: Display>(result: Result<T, E>, context: &str) -> T {
    match result {
        Ok(value) => value,
        Err(error) => panic!("{context}: {error}"),
    }
}

fn write(root: &Path, relative: &str, contents: &str) {
    let path = root.join(relative);
    if let Some(parent) = path.parent() {
        require(fs::create_dir_all(parent), "parent");
    }
    require(fs::write(path, contents), "file");
}

fn request(root: &Path, assets: &str) -> ProjectBuildRequest {
    write(
        root,
        "recite.project.toml",
        &format!("format_version = 1\n\n[discovery]\nsource_roots = [\"dialogue\"]\n\n{assets}"),
    );
    write(
        root,
        "dialogue/main.recite",
        ":: start default speaker=hazel\n> intro@11111111111111111111\n  Hello.\n-> END\n",
    );
    match require(ProjectBuildRequest::prepare(root), "preparation") {
        ProjectBuildPreparation::Ready(request) => *request,
        ProjectBuildPreparation::Rejected { diagnostics } => {
            panic!("unexpected diagnostics: {diagnostics:?}")
        }
        _ => panic!("unknown preparation outcome"),
    }
}

fn candidate(target: &ProjectBuildTarget, byte: u8) -> BuildCandidate {
    BuildCandidate::new(target.target().clone(), vec![byte])
}

fn markers(root: &Path) -> Vec<String> {
    fs::read_dir(root.join("compiled"))
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .filter_map(|entry| entry.file_name().into_string().ok())
        .filter(|name| name.contains(".recite-stage-") && name.ends_with(".tmp"))
        .collect()
}

#[test]
fn clean_prepare_and_commit_replaces_only_on_commit() {
    let temp = require(TempDir::new(), "tempdir");
    let request = request(
        temp.path(),
        "[[scenes]]\nid = \"scene.start\"\nasset = \"compiled/dialogue.recitec\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n",
    );
    let mut publisher = require(ProjectBuildPublisher::new(&request), "publisher");
    let target = &request.targets()[0];
    let bytes = vec![1];
    let prepared = require(
        publisher.prepare(
            request.build_request(),
            &[candidate(target, bytes[0])],
            &BuildControl::new(),
        ),
        "prepare",
    );
    assert!(!temp.path().join("compiled/dialogue.recitec").exists());
    assert_eq!(markers(temp.path()).len(), 1);
    assert_eq!(prepared.identity().candidates()[0].bytes(), &[bytes[0]]);
    assert_eq!(
        publisher.commit(prepared),
        PublishOutcome::Published {
            targets: vec![target.target().clone()]
        }
    );
    assert_eq!(
        require(
            fs::read(temp.path().join("compiled/dialogue.recitec")),
            "published bytes"
        ),
        bytes
    );
    assert!(markers(temp.path()).is_empty());
}

#[test]
fn cancellation_aborts_staging_without_touching_outputs() {
    let temp = require(TempDir::new(), "tempdir");
    let request = request(
        temp.path(),
        "[[scenes]]\nid = \"scene.start\"\nasset = \"compiled/dialogue.recitec\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n",
    );
    let mut publisher = require(ProjectBuildPublisher::new(&request), "publisher");
    let control = BuildControl::new();
    control.cancel();
    assert!(
        publisher
            .prepare(
                request.build_request(),
                &[candidate(&request.targets()[0], 7)],
                &control
            )
            .is_err()
    );
    assert!(!temp.path().join("compiled/dialogue.recitec").exists());
    assert!(markers(temp.path()).is_empty());
}

#[test]
fn failed_stage_write_does_not_replace_outputs() {
    let temp = require(TempDir::new(), "tempdir");
    let request = request(
        temp.path(),
        "[[scenes]]\nid = \"scene.start\"\nasset = \"compiled/blocked/out.recitec\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n",
    );
    require(fs::create_dir_all(temp.path().join("compiled")), "compiled");
    require(
        fs::write(temp.path().join("compiled/blocked"), b"not a directory"),
        "blocking file",
    );
    let mut publisher = require(ProjectBuildPublisher::new(&request), "publisher");
    assert!(
        publisher
            .prepare(
                request.build_request(),
                &[candidate(&request.targets()[0], 4)],
                &BuildControl::new()
            )
            .is_err()
    );
    assert!(!temp.path().join("compiled/blocked/out.recitec").exists());
}

#[test]
fn publisher_binds_the_exact_request_and_orders_candidates() {
    let temp = require(TempDir::new(), "tempdir");
    let request = request(
        temp.path(),
        "[[scenes]]\nid = \"scene.z\"\nasset = \"compiled/z.recitec\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n\n[[scenes]]\nid = \"scene.a\"\nasset = \"compiled/a.recitec\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n",
    );
    let foreign = match require(
        ProjectBuildRequest::prepare_with_generations(
            temp.path(),
            recite_compiler::BuildGeneration::new(2),
            recite_compiler::SnapshotGeneration::initial(),
        ),
        "foreign preparation",
    ) {
        ProjectBuildPreparation::Ready(request) => *request,
        _ => panic!("foreign request was not ready"),
    };
    let mut publisher = require(ProjectBuildPublisher::new(&request), "publisher");
    let mut candidates = request
        .targets()
        .iter()
        .enumerate()
        .map(|(index, target)| candidate(target, index as u8))
        .collect::<Vec<_>>();
    candidates.reverse();
    let prepared = require(
        publisher.prepare(request.build_request(), &candidates, &BuildControl::new()),
        "prepare",
    );
    assert_eq!(
        prepared
            .identity()
            .candidates()
            .iter()
            .map(|candidate| candidate.target().as_str())
            .collect::<Vec<_>>(),
        ["compiled/a.recitec", "compiled/z.recitec"]
    );
    publisher.abort(Some(prepared), PublishAbortReason::Cancelled);
    assert!(markers(temp.path()).is_empty());
    assert!(
        publisher
            .prepare(
                foreign.build_request(),
                &[
                    candidate(&foreign.targets()[0], 8),
                    candidate(&foreign.targets()[1], 9)
                ],
                &BuildControl::new()
            )
            .is_err()
    );
    assert!(markers(temp.path()).is_empty());
}

#[test]
fn failed_replacement_reports_partial_and_keeps_recovery_marker() {
    let temp = require(TempDir::new(), "tempdir");
    let request = request(
        temp.path(),
        "[[scenes]]\nid = \"scene.z\"\nasset = \"compiled/z.recitec\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n\n[[scenes]]\nid = \"scene.a\"\nasset = \"compiled/a.recitec\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n",
    );
    let mut publisher = require(ProjectBuildPublisher::new(&request), "publisher");
    require(
        fs::create_dir_all(temp.path().join("compiled/z.recitec")),
        "blocking directory",
    );
    let candidates = request
        .targets()
        .iter()
        .enumerate()
        .map(|(index, target)| candidate(target, index as u8))
        .collect::<Vec<_>>();
    let prepared = require(
        publisher.prepare(request.build_request(), &candidates, &BuildControl::new()),
        "prepare",
    );
    let outcome = publisher.commit(prepared);
    match outcome {
        PublishOutcome::Partial {
            committed,
            failed,
            remaining,
            recovery,
        } => {
            assert_eq!(committed, [BuildTarget::new("compiled/a.recitec").unwrap()]);
            assert_eq!(failed, BuildTarget::new("compiled/z.recitec").unwrap());
            assert!(remaining.is_empty());
            assert_eq!(
                recovery.targets(),
                &[
                    BuildTarget::new("compiled/a.recitec").unwrap(),
                    BuildTarget::new("compiled/z.recitec").unwrap(),
                ]
            );
        }
        other => panic!("unexpected outcome: {other:?}"),
    }
    assert!(temp.path().join("compiled/a.recitec").is_file());
    assert!(temp.path().join("compiled/z.recitec").is_dir());
    assert_eq!(markers(temp.path()).len(), 1);
}

#[test]
fn unsafe_target_forms_and_source_alias_are_rejected() {
    for asset in [
        "../outside.recitec",
        "/tmp/outside.recitec",
        "C:/outside.recitec",
        "compiled//outside.recitec",
    ] {
        let temp = require(TempDir::new(), "tempdir");
        let request = request(
            temp.path(),
            &format!(
                "[[scenes]]\nid = \"scene.start\"\nasset = \"{asset}\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n"
            ),
        );
        assert!(ProjectBuildPublisher::new(&request).is_err(), "{asset}");
    }

    let temp = require(TempDir::new(), "tempdir");
    let request = request(
        temp.path(),
        "[[scenes]]\nid = \"scene.start\"\nasset = \"dialogue/main.recite\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n",
    );
    assert!(ProjectBuildPublisher::new(&request).is_err());
}

#[cfg(unix)]
#[test]
fn outside_output_symlink_is_rejected_before_staging() {
    use std::os::unix::fs::symlink;

    let temp = require(TempDir::new(), "tempdir");
    let outside = require(TempDir::new(), "outside");
    let request = request(
        temp.path(),
        "[[scenes]]\nid = \"scene.start\"\nasset = \"compiled/dialogue.recitec\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n",
    );
    require(fs::create_dir_all(temp.path().join("compiled")), "compiled");
    require(
        symlink(
            outside.path(),
            temp.path().join("compiled/dialogue.recitec"),
        ),
        "outside symlink",
    );
    assert!(ProjectBuildPublisher::new(&request).is_err());
    assert!(markers(temp.path()).is_empty());
}

#[cfg(unix)]
#[test]
fn inside_output_symlink_remains_within_the_project_boundary() {
    use std::os::unix::fs::symlink;

    let temp = require(TempDir::new(), "tempdir");
    let request = request(
        temp.path(),
        "[[scenes]]\nid = \"scene.start\"\nasset = \"compiled/link/dialogue.recitec\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n",
    );
    require(fs::create_dir_all(temp.path().join("compiled")), "compiled");
    require(fs::create_dir_all(temp.path().join("inside")), "inside");
    require(
        symlink(
            temp.path().join("inside"),
            temp.path().join("compiled/link"),
        ),
        "inside symlink",
    );
    assert!(ProjectBuildPublisher::new(&request).is_ok());
}

#[cfg(windows)]
#[test]
fn existing_windows_output_failure_is_reported_without_removal() {
    let temp = require(TempDir::new(), "tempdir");
    let request = request(
        temp.path(),
        "[[scenes]]\nid = \"scene.start\"\nasset = \"compiled/dialogue.recitec\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n",
    );
    let output = temp.path().join("compiled/dialogue.recitec");
    require(fs::create_dir_all(temp.path().join("compiled")), "compiled");
    require(fs::write(&output, b"old"), "old output");
    let mut publisher = require(ProjectBuildPublisher::new(&request), "publisher");
    let prepared = require(
        publisher.prepare(
            request.build_request(),
            &[candidate(&request.targets()[0], 8)],
            &BuildControl::new(),
        ),
        "prepare",
    );
    assert!(matches!(
        publisher.commit(prepared),
        PublishOutcome::Partial { .. }
    ));
    assert_eq!(require(fs::read(output), "output"), b"old");
}
