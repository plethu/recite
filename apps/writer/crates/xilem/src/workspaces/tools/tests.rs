use super::*;
use crate::{
    tests::support,
    workspaces::{Open, Screen, opening},
};
use recite_core::schema::SchemaSource;
use std::{fs, path::Path};
use tokio::sync::mpsc;
#[path = "visual/tests.rs"]
mod visual;

pub(super) fn fixture() -> (tempfile::TempDir, Writer, mpsc::UnboundedReceiver<Job>) {
    fixture_source(None)
}
fn fixture_source(
    scene: Option<&str>,
) -> (tempfile::TempDir, Writer, mpsc::UnboundedReceiver<Job>) {
    let directory = super::super::tests::fixture().unwrap();
    if let Some(scene) = scene {
        fs::write(directory.path().join("scene.recite"), scene).unwrap();
    }
    let source =
        "schema_version=1\n[producer]\nid='dialogue'\n[speakers.mara]\ndisplay_name='Mara'\n";
    let export = |source: &str| {
        SchemaSource::load_str("schema.toml", source)
            .source
            .unwrap()
            .export_json()
    };
    fs::write(directory.path().join("schema.json"), export(source)).unwrap();
    fs::write(directory.path().join("schema.toml"), source).unwrap();
    fs::write(
        directory.path().join("fixture.json"),
        export(&source.replace("Mara", "Updated")),
    )
    .unwrap();
    fs::write(
        directory
            .path()
            .join(recite_config::PRODUCER_REGISTRATION_FILE),
        r#"
version=1
[producer]
kind="standalone"
id="dialogue"
[generate]
program="/bin/sh"
args=["-c", "cp fixture.json \"$1\"", "producer", "{output}"]
"#,
    )
    .unwrap();
    let mut writer = support::open(&directory).unwrap();
    let mut prepared = opening::Work::capture(&writer, Open::Declarations)
        .unwrap()
        .execute()
        .unwrap();
    writer.accept_project_workspace(&mut prepared).unwrap();
    let (sender, jobs) = mpsc::unbounded_channel();
    writer.workspaces.tools.sender = Some(sender);
    (directory, writer, jobs)
}

#[cfg(unix)]
#[test]
fn schema_refresh_retires_stale_rule_controls_and_preserves_the_repairable_draft_policy() {
    let source =
        ":: start default\n? reply@11111111111111111111 requires=(missing())\n  Hello.\n  -> END\n";
    let (_directory, mut writer, mut jobs) = fixture_source(Some(source));
    let (_, model) = writer.session.parts_mut().unwrap();
    let mut projection = model.open_reply_rules("11111111111111111111").unwrap();
    model
        .stage_reply_rules(&mut projection, |rules| rules.condition = None)
        .unwrap();
    let draft = model.draft().to_owned();
    let policy = model.source_validation();
    writer.install_rules(&mut Some(projection));
    assert!(writer.rules.is_some());
    writer.regenerate_schema();
    writer.return_to_writing();
    finish(&mut writer, &mut jobs);
    assert!(writer.rules.is_none(), "{}", writer.status);
    assert_eq!(writer.session.model().draft(), draft);
    assert_eq!(writer.session.model().source_validation(), policy);
    assert!(writer.source_repair_pending());
    assert!(writer.status.contains("staged reply draft"));
}
pub(super) fn finish(writer: &mut Writer, jobs: &mut mpsc::UnboundedReceiver<Job>) {
    for _ in 0..200 {
        if let Ok(job) = jobs.try_recv() {
            let mut completed = job.execute();
            loop {
                let request = completed.request.clone();
                let retirement = writer.accept_tool(completed);
                match retirement.finish(request) {
                    Some(next) => completed = next,
                    None => break,
                }
            }
        }
        writer.poll_project_jobs();
        if !writer.project_jobs_busy() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("Project tools did not retire: {}", writer.status);
}

#[cfg(unix)]
#[test]
fn hidden_generation_retains_newer_typing_undo_and_close_guard_until_analysis_is_adopted() {
    let (directory, mut writer, mut jobs) = fixture();
    writer.change(|model| {
        model.set_draft("Applied prose.".into());
        model.apply()
    });
    writer.regenerate_schema();
    writer.return_to_writing();
    writer.change(|model| {
        model.set_draft("Newer typing.".into());
        Ok(())
    });
    writer.request_close();
    assert!(writer.phase == Phase::Writing);
    finish(&mut writer, &mut jobs);
    assert!(writer.workspaces.screen == Screen::Write);
    assert_eq!(writer.session.model().draft(), "Newer typing.");
    assert!(
        writer
            .session
            .model()
            .document()
            .source()
            .contains("Applied prose")
    );
    assert!(
        fs::read_to_string(directory.path().join("schema.json"))
            .unwrap()
            .contains("Updated")
    );
    assert!(
        writer.workspaces.context_refresh.is_none(),
        "{}",
        writer.status
    );
    let (_, model) = writer.session.parts_mut().unwrap();
    model.discard();
    model.undo().unwrap();
    assert!(model.document().source().contains("Hello"));
}

#[cfg(unix)]
#[test]
fn published_schema_survives_failed_context_refresh_and_can_be_retried() {
    let (directory, mut writer, mut jobs) = fixture();
    writer.regenerate_schema();
    writer.accept_tool(jobs.try_recv().unwrap().execute());
    fs::write(
        directory.path().join("recite.project.toml"),
        "invalid TOML [",
    )
    .unwrap();
    finish(&mut writer, &mut jobs);
    assert!(
        writer
            .status
            .contains("Schema published; context refresh failed"),
        "{}",
        writer.status
    );
    assert!(writer.workspaces.context_refresh.is_some());
    assert!(
        fs::read_to_string(directory.path().join("schema.json"))
            .unwrap()
            .contains("Updated")
    );
    assert!(
        writer
            .declaration_session()
            .unwrap()
            .last_valid_schema()
            .speakers["mara"]
            .display_name
            .as_deref()
            == Some("Updated")
    );
    fs::write(
        directory.path().join("recite.project.toml"),
        "format_version=1\n[project]\nschema='schema.json'\n",
    )
    .unwrap();
    writer.refresh_project_context();
    finish(&mut writer, &mut jobs);
    assert!(
        writer.workspaces.context_refresh.is_none(),
        "{}",
        writer.status
    );
}

#[cfg(unix)]
#[test]
fn dispatch_failure_and_startup_cancellation_resolve_the_canonical_busy_state() {
    let (_directory, mut writer, mut jobs) = fixture();
    writer.workspaces.tools.sender = None;
    writer.regenerate_schema();
    assert!(!writer.project_jobs_busy());
    assert!(!writer.declaration_session().unwrap().busy());
    let (sender, received) = mpsc::unbounded_channel();
    jobs.close();
    jobs = received;
    writer.workspaces.tools.sender = Some(sender);
    writer.regenerate_schema();
    writer.cancel_schema_generation();
    assert!(writer.project_jobs_busy());
    finish(&mut writer, &mut jobs);
    assert!(writer.status.contains("cancelled"), "{}", writer.status);
    assert!(!writer.declaration_session().unwrap().busy());
    assert!(
        writer
            .declaration_session_mut()
            .unwrap()
            .bind(Path::new("missing"))
            .is_err()
    );
}
