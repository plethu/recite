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
    format!(":: start default speaker=hazel\n> intro@11111111111111111111\n  {text}\n-> END\n")
}

fn schema_project() -> &'static str {
    "format_version = 1\n\n[project]\nschema = \"schema.json\"\n\n[[scenes]]\nid = \"scene.start\"\nasset = \"compiled/dialogue.recitec\"\nblock = \"start\"\nparticipants = [\"hazel\"]\n"
}

#[test]
fn post_publish_source_change_is_stale_not_fresh() {
    let temp = TempDir::new().expect("tempdir");
    write(temp.path(), "recite.project.toml", &project(true));
    write(temp.path(), "dialogue/main.recite", &source("Before."));
    let mut state = WatchState::new(temp.path().to_owned());
    let messages = Messages::load(&UiLocale::default()).expect("messages");
    let control = BuildControl::new();
    let mut stderr = Vec::new();
    let status =
        build_once_with_post_publish_hook(&mut state, &mut stderr, &messages, &control, || {
            write(temp.path(), "dialogue/main.recite", &source("After."))
        })
        .expect("build");

    assert!(matches!(status, BuildStatus::Stale { asset_count: 1 }));
    assert!(temp.path().join("compiled/dialogue.recitec").is_file());
    assert!(
        String::from_utf8(stderr)
            .expect("stderr")
            .contains("RECITE_FRESH001")
    );
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
    let messages = Messages::load(&UiLocale::default()).expect("messages");
    let control = BuildControl::new();
    let mut stderr = Vec::new();
    let status =
        build_once_with_post_publish_hook(&mut state, &mut stderr, &messages, &control, || {
            write(
                temp.path(),
                "schema.json",
                r#"{"schema_version":1,"speakers":{"hazel":{"display_name":"After"}}}"#,
            )
        })
        .expect("build");

    assert!(matches!(status, BuildStatus::Stale { asset_count: 1 }));
    assert!(
        String::from_utf8(stderr)
            .expect("stderr")
            .contains("RECITE_FRESH002")
    );
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
