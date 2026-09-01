use super::{BuildStatus, project, run_hook, schema_project, source, write};
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
