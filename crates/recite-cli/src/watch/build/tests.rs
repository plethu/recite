use std::fs;
use std::path::Path;

use recite_compiler::{BuildControl, BuildGeneration, PublishNotAttemptedReason};
use tempfile::TempDir;

use super::super::events::WatchState;
use super::{BuildStatus, build_once_with_control, build_once_with_post_publish_hook};
use crate::i18n::{Messages, UiLocale};

fn write(root: &Path, name: &str, text: &str) {
    let path = root.join(name);
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent).expect("parent");
    }
    fs::write(path, text).expect("file");
}

fn project(with_target: bool) -> String {
    if with_target {
        "format_version = 1\n\n[[scenes]]\nid = \"scene.start\"\nasset = \"compiled/dialogue.recitec\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n".to_owned()
    } else {
        "format_version = 1\n".to_owned()
    }
}

fn source(text: &str) -> String {
    source_with("11111111111111111111", text)
}

fn source_with(id: &str, text: &str) -> String {
    source_with_block("start default speaker=hazel", id, text)
}

fn source_with_block(header: &str, id: &str, text: &str) -> String {
    format!(":: {header}\n> intro@{id}\n  {text}\n-> END\n")
}

fn schema_project() -> &'static str {
    "format_version = 1\n\n[project]\nschema = \"schema.json\"\n\n[[scenes]]\nid = \"scene.start\"\nasset = \"compiled/dialogue.recitec\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n"
}

fn run_hook<F: FnOnce()>(state: &mut WatchState, hook: F) -> (BuildStatus, String) {
    let messages = Messages::load(&UiLocale::default()).expect("messages");
    let control = BuildControl::new();
    let mut stderr = Vec::new();
    let status = build_once_with_post_publish_hook(state, &mut stderr, &messages, &control, hook)
        .expect("build");
    (status, String::from_utf8(stderr).expect("stderr"))
}

#[test]
fn post_publish_source_change_is_stale_not_fresh() {
    let temp = TempDir::new().expect("tempdir");
    write(temp.path(), "recite.project.toml", &project(true));
    write(temp.path(), "dialogue/main.recite", &source("Before."));
    let mut state = WatchState::new(temp.path().to_owned());
    let (status, _stderr) = run_hook(&mut state, || {
        write(temp.path(), "dialogue/main.recite", &source("After."))
    });

    assert!(matches!(status, BuildStatus::Stale { asset_count: 1 }));
    assert!(temp.path().join("compiled/dialogue.recitec").is_file());
}

#[test]
fn post_publish_schema_change_is_stale_not_fresh() {
    let temp = TempDir::new().expect("tempdir");
    write(temp.path(), "recite.project.toml", schema_project());
    write(
        temp.path(),
        "schema.json",
        r#"{"schema_version":1,"speakers":{"hazel":{"display_name":"Hazel"}}}"#,
    );
    write(temp.path(), "dialogue/main.recite", &source("Before."));
    let mut state = WatchState::new(temp.path().to_owned());
    let (status, _stderr) = run_hook(&mut state, || {
        write(
            temp.path(),
            "schema.json",
            r#"{"schema_version":1,"speakers":{"hazel":{"display_name":"After"}}}"#,
        )
    });

    assert!(
        matches!(status, BuildStatus::Stale { asset_count: 1 }),
        "{status:?}"
    );
}

#[test]
fn post_publish_manifest_changes_are_stale_not_fresh() {
    for manifest in [
        "format_version = 1\n\n[[scenes]]\nid = \"scene.changed\"\nasset = \"compiled/dialogue.recitec\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n",
        "format_version = 1\n",
    ] {
        let temp = TempDir::new().expect("tempdir");
        write(temp.path(), "recite.project.toml", &project(true));
        write(temp.path(), "dialogue/main.recite", &source("Before."));
        let mut state = WatchState::new(temp.path().to_owned());
        let (status, _) = run_hook(&mut state, || {
            write(temp.path(), "recite.project.toml", manifest)
        });
        assert!(matches!(status, BuildStatus::Stale { asset_count: 1 }));
        assert!(temp.path().join("compiled/dialogue.recitec").is_file());
    }
}

#[test]
fn post_publish_target_set_change_is_stale_not_fresh() {
    let temp = TempDir::new().expect("tempdir");
    write(temp.path(), "recite.project.toml", &project(true));
    write(temp.path(), "dialogue/main.recite", &source("Before."));
    let mut state = WatchState::new(temp.path().to_owned());
    let (status, _) = run_hook(&mut state, || {
        write(
            temp.path(),
            "recite.project.toml",
            "format_version = 1\n\n[[scenes]]\nid = \"scene.start\"\nasset = \"compiled/dialogue.recitec\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n\n[[scenes]]\nid = \"scene.second\"\nasset = \"compiled/second.recitec\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n",
        )
    });
    assert!(matches!(status, BuildStatus::Stale { asset_count: 1 }));
}

#[test]
fn newly_discovered_source_is_stale_not_fresh() {
    let temp = TempDir::new().expect("tempdir");
    write(temp.path(), "recite.project.toml", &project(true));
    write(temp.path(), "dialogue/main.recite", &source("Before."));
    let mut state = WatchState::new(temp.path().to_owned());
    let (status, _) = run_hook(&mut state, || {
        write(
            temp.path(),
            "dialogue/new.recite",
            &source_with_block("other", "22222222222222222222", "New source."),
        )
    });
    assert!(
        matches!(status, BuildStatus::Stale { asset_count: 1 }),
        "{status:?}"
    );
}

#[test]
fn unchanged_request_is_fresh_after_post_publish_assessment() {
    let temp = TempDir::new().expect("tempdir");
    write(temp.path(), "recite.project.toml", &project(true));
    write(temp.path(), "dialogue/main.recite", &source("Unchanged."));
    let mut state = WatchState::new(temp.path().to_owned());

    let (status, stderr) = run_hook(&mut state, || {});

    assert_eq!(status, BuildStatus::Fresh { asset_count: 1 });
    assert!(stderr.is_empty());
}

#[test]
fn empty_target_build_honours_cancellation_and_supersession() {
    for cancellation in [None, Some(BuildGeneration::new(1))] {
        let temp = TempDir::new().expect("tempdir");
        write(temp.path(), "recite.project.toml", &project(false));
        write(temp.path(), "dialogue/main.recite", &source("No output."));
        let mut state = WatchState::new(temp.path().to_owned());
        let control = BuildControl::new();
        match cancellation {
            Some(generation) => control.supersede(generation),
            None => control.cancel(),
        }
        let messages = Messages::load(&UiLocale::default()).expect("messages");
        let mut stderr = Vec::new();
        let result =
            build_once_with_control(&mut state, &mut stderr, &messages, &control).expect("build");
        let expected = cancellation.map_or(PublishNotAttemptedReason::Cancelled, |_| {
            PublishNotAttemptedReason::Superseded
        });
        match result {
            BuildStatus::PublicationFailure {
                status, outcome, ..
            } => {
                assert_eq!(
                    status,
                    if cancellation.is_some() {
                        recite_compiler::BuildTerminalStatus::Superseded
                    } else {
                        recite_compiler::BuildTerminalStatus::Cancelled
                    }
                );
                assert_eq!(
                    outcome,
                    recite_compiler::PublishOutcome::NotAttempted { reason: expected }
                );
            }
            other => panic!("unexpected empty-target result: {other:?}"),
        }
    }
}
