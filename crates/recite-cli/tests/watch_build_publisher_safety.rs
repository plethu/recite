use std::fmt::Display;
use std::fs;
use std::path::Path;

use recite_cli::watch::{
    ProjectBuildPreparation, ProjectBuildPublisher, ProjectBuildPublisherError,
    ProjectBuildRequest, TargetMapError, TargetPathError,
};
use recite_compiler::{BuildCandidate, BuildControl, BuildPublisher, PublishAbortReason};
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

fn markers(root: &Path) -> Vec<std::path::PathBuf> {
    fs::read_dir(root.join("compiled"))
        .ok()
        .into_iter()
        .flatten()
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| name.contains(".recite-stage-") && name.ends_with(".tmp"))
        })
        .collect()
}

#[test]
fn marker_collision_retries_and_abort_cleans_new_marker() {
    let temp = require(TempDir::new(), "tempdir");
    let request = request(
        temp.path(),
        "[[scenes]]\nid = \"scene.start\"\nasset = \"compiled/dialogue.recitec\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n",
    );
    let mut publisher = require(ProjectBuildPublisher::new(&request), "publisher");
    let candidate = BuildCandidate::new(request.targets()[0].target().clone(), [1]);
    let first = require(
        publisher.prepare(
            request.build_request(),
            std::slice::from_ref(&candidate),
            &BuildControl::new(),
        ),
        "first prepare",
    );
    let first_marker = match markers(temp.path()).into_iter().next() {
        Some(marker) => marker,
        None => panic!("first stage marker was not created"),
    };
    publisher.abort(Some(first), PublishAbortReason::Cancelled);
    require(fs::write(&first_marker, b"orphan"), "orphan marker");

    let second = require(
        publisher.prepare(
            request.build_request(),
            std::slice::from_ref(&candidate),
            &BuildControl::new(),
        ),
        "second prepare",
    );
    let second_markers = markers(temp.path());
    assert_eq!(second_markers.len(), 2);
    assert!(second_markers.iter().any(|path| path != &first_marker));
    publisher.abort(Some(second), PublishAbortReason::Cancelled);
    require(fs::remove_file(first_marker), "orphan cleanup");
    assert!(markers(temp.path()).is_empty());
    assert!(publisher.recovery().is_empty());
}

#[test]
fn blocked_output_parent_is_rejected_without_writing() {
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
    assert!(ProjectBuildPublisher::new(&request).is_err());
    assert!(!temp.path().join("compiled/blocked/out.recitec").exists());
}

#[test]
fn target_path_failures_keep_their_exact_typed_category() {
    for (asset, expected) in [
        ("/tmp/outside.recitec", TargetPathError::Absolute),
        ("../outside.recitec", TargetPathError::Parent),
        ("C:/outside.recitec", TargetPathError::PlatformAmbiguous),
    ] {
        let temp = require(TempDir::new(), "tempdir");
        let request = request(
            temp.path(),
            &format!(
                "[[scenes]]\nid = \"scene.start\"\nasset = \"{asset}\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n"
            ),
        );
        let error = match ProjectBuildPublisher::new(&request) {
            Ok(_) => panic!("unsafe target was accepted: {asset}"),
            Err(error) => error,
        };
        assert!(matches!(
            error,
            ProjectBuildPublisherError::Targets(TargetMapError::InvalidTarget { reason, .. })
                if reason == expected
        ));
    }
}

#[cfg(unix)]
#[test]
fn symlink_alias_destinations_are_rejected_before_staging() {
    use std::os::unix::fs::symlink;

    let temp = require(TempDir::new(), "tempdir");
    let request = request(
        temp.path(),
        "[[scenes]]\nid = \"scene.real\"\nasset = \"compiled/real.recitec\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n\n[[scenes]]\nid = \"scene.alias\"\nasset = \"compiled/alias.recitec\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n",
    );
    require(fs::create_dir_all(temp.path().join("compiled")), "compiled");
    require(
        fs::write(temp.path().join("compiled/real.recitec"), b"old"),
        "real output",
    );
    require(
        symlink(
            temp.path().join("real.recitec"),
            temp.path().join("compiled/alias.recitec"),
        ),
        "alias symlink",
    );
    assert!(ProjectBuildPublisher::new(&request).is_err());
}

#[cfg(unix)]
#[test]
fn output_parent_symlink_swap_is_refused_at_commit() {
    use std::os::unix::fs::symlink;

    let temp = require(TempDir::new(), "tempdir");
    let outside = require(TempDir::new(), "outside");
    let request = request(
        temp.path(),
        "[[scenes]]\nid = \"scene.start\"\nasset = \"compiled/dialogue.recitec\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n",
    );
    let mut publisher = require(ProjectBuildPublisher::new(&request), "publisher");
    let candidate = BuildCandidate::new(request.targets()[0].target().clone(), [3]);
    let prepared = require(
        publisher.prepare(
            request.build_request(),
            std::slice::from_ref(&candidate),
            &BuildControl::new(),
        ),
        "prepare",
    );
    require(
        fs::rename(
            temp.path().join("compiled"),
            temp.path().join("compiled.saved"),
        ),
        "move staged parent",
    );
    require(
        symlink(outside.path(), temp.path().join("compiled")),
        "swap output parent",
    );
    let outcome = publisher.commit(prepared);
    assert!(matches!(
        outcome,
        recite_compiler::PublishOutcome::Partial { .. }
    ));
    assert!(!outside.path().join("dialogue.recitec").exists());
    assert_eq!(publisher.recovery().len(), 1);
}

#[cfg(windows)]
#[test]
fn case_alias_destinations_are_rejected_before_staging() {
    let temp = require(TempDir::new(), "tempdir");
    let request = request(
        temp.path(),
        "[[scenes]]\nid = \"scene.lower\"\nasset = \"compiled/café.recitec\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n\n[[scenes]]\nid = \"scene.upper\"\nasset = \"compiled/CAFÉ.recitec\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n",
    );
    assert!(ProjectBuildPublisher::new(&request).is_err());
}

#[cfg(target_os = "macos")]
#[test]
fn macos_case_alias_destinations_are_rejected_before_staging() {
    let temp = require(TempDir::new(), "tempdir");
    let request = request(
        temp.path(),
        "[[scenes]]\nid = \"scene.lower\"\nasset = \"compiled/café.recitec\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n\n[[scenes]]\nid = \"scene.upper\"\nasset = \"compiled/CAFÉ.recitec\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n",
    );
    assert!(ProjectBuildPublisher::new(&request).is_err());
}
