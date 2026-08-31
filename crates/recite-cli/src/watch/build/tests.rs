use std::fs;
use std::path::Path;

use recite_compiler::{
    BuildCheckError, BuildControl, BuildFailureReason, BuildGeneration, BuildResultFailure,
    BuildTarget, BuildTerminalStatus, PublishNotAttemptedReason, PublishOutcome, PublishRefusal,
    RecoveryNeeded,
};
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

fn alternate_messages(replacements: &[(&str, &str)]) -> Messages {
    let mut resource = recite_ui::DEFAULT_RESOURCE.to_owned();
    for (id, replacement) in replacements {
        let mut found = false;
        let lines = resource
            .lines()
            .map(|line| {
                if line.starts_with(&format!("{id} = ")) {
                    found = true;
                    format!("{id} = {replacement}")
                } else {
                    line.to_owned()
                }
            })
            .collect::<Vec<_>>();
        assert!(found, "missing resource {id}");
        resource = format!("{}\n", lines.join("\n"));
    }
    recite_ui::UiContract::default()
        .validate(&resource)
        .expect("alternate resource contract");
    Messages::from_resources(
        "en-GB".parse().expect("locale"),
        [
            (
                "en-US".parse().expect("locale"),
                recite_ui::DEFAULT_RESOURCE.to_owned(),
            ),
            ("en-GB".parse().expect("locale"), resource),
        ],
    )
    .expect("alternate messages")
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

#[test]
fn publication_failure_uses_alternate_ids_and_typed_arguments() {
    let messages = alternate_messages(&[
        (
            "watch-build-failed-partial-with-failure",
            "alt-partial status={$status} failed={$failed} recovery={$recovery} failure={$failure}",
        ),
        ("watch-build-status-failed", "alt-status-failed"),
        ("watch-build-failure-engine-host", "alt-engine-host"),
        (
            "watch-build-failed-indeterminate",
            "alt-indeterminate status={$status} recovery={$recovery}",
        ),
        ("watch-build-status-cancelled", "alt-status-cancelled"),
    ]);
    let target = BuildTarget::new("compiled/main.recitec".to_owned()).expect("target");
    let recovery = RecoveryNeeded::for_targets(vec![target.clone()]);

    assert_eq!(
        super::format_failure(
            &messages,
            BuildTerminalStatus::Failed,
            Some(&BuildResultFailure::Engine {
                reason: BuildFailureReason::Host,
            }),
            &PublishOutcome::Partial {
                committed: Vec::new(),
                failed: target.clone(),
                remaining: Vec::new(),
                recovery,
            },
        ),
        "alt-partial status=alt-status-failed failed=compiled/main.recitec recovery=compiled/main.recitec failure=alt-engine-host"
    );
    assert_eq!(
        super::format_failure(
            &messages,
            BuildTerminalStatus::Cancelled,
            None,
            &PublishOutcome::Indeterminate {
                attempted: vec![target.clone()],
                recovery: RecoveryNeeded::for_targets(vec![target]),
            },
        ),
        "alt-indeterminate status=alt-status-cancelled recovery=compiled/main.recitec"
    );
}

#[test]
fn publication_failure_localizes_reason_categories_without_debug_output() {
    let messages = alternate_messages(&[
        (
            "watch-build-failed-refused-with-failure",
            "alt-refused {$status} {$reason} {$failure}",
        ),
        ("watch-build-status-stale", "alt-status-stale"),
        (
            "watch-build-failure-refusal-stale-fingerprints",
            "alt-stale-fingerprints",
        ),
        (
            "watch-build-failure-check-request-mismatch",
            "alt-request-mismatch",
        ),
        (
            "watch-build-failed-not-attempted",
            "alt-not-attempted {$status} {$reason}",
        ),
        ("watch-build-status-superseded", "alt-status-superseded"),
        (
            "watch-build-failure-not-attempted-superseded",
            "alt-superseded",
        ),
    ]);

    assert_eq!(
        super::format_failure(
            &messages,
            BuildTerminalStatus::Stale,
            Some(&BuildResultFailure::Check(BuildCheckError::RequestMismatch,)),
            &PublishOutcome::Refused {
                reason: PublishRefusal::StaleFingerprints,
            },
        ),
        "alt-refused alt-status-stale alt-stale-fingerprints alt-request-mismatch"
    );
    assert_eq!(
        super::format_failure(
            &messages,
            BuildTerminalStatus::Superseded,
            None,
            &PublishOutcome::NotAttempted {
                reason: PublishNotAttemptedReason::Superseded,
            },
        ),
        "alt-not-attempted alt-status-superseded alt-superseded"
    );
}
