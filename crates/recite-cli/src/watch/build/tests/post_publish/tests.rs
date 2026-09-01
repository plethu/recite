use super::{
    BuildStatus, alternate_messages, build_once_with_post_publish_hook, project, run_hook,
    run_hook_with_messages, schema_project, source, write,
};
use tempfile::TempDir;

#[test]
fn post_publish_rejected_source_is_stale_and_keeps_diagnostics() {
    let temp = TempDir::new().expect("tempdir");
    write(temp.path(), "recite.project.toml", &project(true));
    write(temp.path(), "dialogue/main.recite", &source("Before."));
    let mut state = super::super::super::events::WatchState::new(temp.path().to_owned());

    let (status, stderr) = run_hook(&mut state, || {
        write(
            temp.path(),
            "dialogue/main.recite",
            ":: start\n:if broken(\n  malformed\n",
        )
    });

    assert!(
        matches!(status, BuildStatus::Stale { asset_count: 1, .. }),
        "{status:?}"
    );
    assert!(
        stderr.contains("error RECITE_PARSE013 dialogue/main.recite:2:12"),
        "{stderr}"
    );
    let result = state.coordinator.state().result().expect("shared result");
    assert_eq!(result.status(), recite_compiler::BuildTerminalStatus::Stale);
    assert_eq!(
        result.freshness().status(),
        recite_compiler::FreshnessStatus::Stale
    );
    assert!(matches!(
        result.publish(),
        recite_compiler::PublishOutcome::Published { .. }
    ));
    assert!(temp.path().join("compiled/dialogue.recitec").is_file());
}

#[test]
fn post_publish_rejected_schema_is_stale_and_keeps_diagnostics() {
    let temp = TempDir::new().expect("tempdir");
    write(temp.path(), "recite.project.toml", schema_project());
    write(
        temp.path(),
        "schema.json",
        r#"{"schema_version":1,"speakers":{"hazel":{"display_name":"Hazel"}}}"#,
    );
    write(temp.path(), "dialogue/main.recite", &source("Before."));
    let mut state = super::super::super::events::WatchState::new(temp.path().to_owned());

    let (status, stderr) = run_hook(&mut state, || {
        write(temp.path(), "schema.json", r#"{"schema_version":"one"}"#)
    });

    assert!(
        matches!(status, BuildStatus::Stale { asset_count: 1, .. }),
        "{status:?}"
    );
    assert!(stderr.contains("error RECITE_SCHEMA001"), "{stderr}");
    assert!(temp.path().join("compiled/dialogue.recitec").is_file());
}

#[test]
fn post_publish_rejection_uses_localized_waiting_message() {
    let temp = TempDir::new().expect("tempdir");
    write(temp.path(), "recite.project.toml", &project(true));
    write(temp.path(), "dialogue/main.recite", &source("Before."));
    let mut state = super::super::super::events::WatchState::new(temp.path().to_owned());
    let messages = alternate_messages(&[("watch-build-failed-waiting", "alt-waiting")]);

    let (status, stderr) = run_hook_with_messages(&mut state, &messages, || {
        write(
            temp.path(),
            "dialogue/main.recite",
            ":: start\n:if broken(\n  malformed\n",
        )
    });
    let mut rendered = stderr.into_bytes();
    crate::watch::report_build_result(&mut rendered, Ok(status), &messages)
        .expect("localized status report");

    assert!(
        String::from_utf8(rendered)
            .expect("utf8")
            .contains("alt-waiting")
    );
}

#[test]
fn post_publish_recheck_error_updates_shared_state_and_keeps_localized_error() {
    let temp = TempDir::new().expect("tempdir");
    write(temp.path(), "recite.project.toml", &project(true));
    write(temp.path(), "dialogue/main.recite", &source("Before."));
    let mut state = super::super::super::events::WatchState::new(temp.path().to_owned());
    let messages = alternate_messages(&[("watch-build-failed", "alt-error {$error}")]);
    let asset = temp.path().join("compiled/dialogue.recitec");

    let control = recite_compiler::BuildControl::new();
    let mut output = Vec::new();
    let result =
        build_once_with_post_publish_hook(&mut state, &mut output, &messages, &control, || {
            std::fs::remove_file(&asset).expect("remove published asset");
            std::fs::create_dir(&asset).expect("replace asset with directory");
        })
        .expect_err("recheck error");
    let mut rendered = Vec::new();
    crate::watch::report_build_result(&mut rendered, Err(result), &messages)
        .expect("localized report");
    assert!(
        String::from_utf8(rendered)
            .expect("utf8")
            .contains("alt-error")
    );
    let result = state.coordinator.state().result().expect("shared result");
    assert_eq!(
        result.status(),
        recite_compiler::BuildTerminalStatus::Failed
    );
    assert_eq!(
        result.freshness().status(),
        recite_compiler::FreshnessStatus::Unknown
    );
    assert!(matches!(
        result.failure(),
        Some(recite_compiler::BuildResultFailure::Freshness { .. })
    ));
    assert!(matches!(
        result.publish(),
        recite_compiler::PublishOutcome::Published { .. }
    ));
}
