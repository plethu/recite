use super::*;
use crate::{
    tests::support,
    workspaces::{Open, Screen, opening, tests::fixture as project_fixture},
};
use std::fs;
use tokio::sync::mpsc;
#[path = "visual/tests.rs"]
mod visual;
#[path = "workflow/tests.rs"]
mod workflow;

fn fixture() -> (tempfile::TempDir, Writer, mpsc::UnboundedReceiver<Job>) {
    let directory = project_fixture().unwrap();
    let mut writer = support::open(&directory).unwrap();
    let mut prepared = opening::Work::capture(&writer, Open::Settings)
        .unwrap()
        .execute()
        .unwrap();
    writer.accept_project_workspace(&mut prepared).unwrap();
    let (sender, jobs) = mpsc::unbounded_channel();
    writer.workspaces.tools.sender = Some(sender);
    (directory, writer, jobs)
}
fn import(writer: &mut Writer, jobs: &mut mpsc::UnboundedReceiver<Job>, path: PathBuf) {
    writer.project_settings_action(SettingsAction::ImportDraft(path));
    super::super::tests::finish(writer, jobs);
}
#[test]
fn settings_publish_and_refresh_preserve_newer_manuscript_and_accepted_history() {
    let (directory, mut writer, mut jobs) = fixture();
    let manifest = directory.path().join("recite.project.toml");
    let original = fs::read_to_string(&manifest).unwrap();
    let candidate = original.replace("content_set='trial'", "content_set='changed'");
    let artifact = directory.path().join("draft.toml");
    fs::write(&artifact, &candidate).unwrap();
    writer.change(|model| {
        model.set_draft("Applied prose.".into());
        model.apply()
    });
    import(&mut writer, &mut jobs, artifact);
    assert_eq!(fs::read_to_string(&manifest).unwrap(), original);
    assert_eq!(
        writer.project_settings().unwrap().reviewed_source(),
        Some(candidate.as_str())
    );
    writer.apply_project_settings();
    writer.return_to_writing();
    writer.change(|model| {
        model.set_draft("Newer typing.".into());
        Ok(())
    });
    writer.request_close();
    assert!(writer.phase == Phase::Writing);
    super::super::tests::finish(&mut writer, &mut jobs);
    assert!(writer.workspaces.screen == Screen::Write);
    assert_eq!(writer.session.model().draft(), "Newer typing.");
    assert_eq!(fs::read_to_string(&manifest).unwrap(), candidate);
    assert!(
        !writer
            .session
            .project()
            .unwrap()
            .files()
            .configuration_refresh_required()
    );
    assert!(
        writer
            .status
            .contains("Project settings saved; project analysis refreshed"),
        "{}",
        writer.status
    );
    assert!(
        !fs::read_to_string(directory.path().join("scene.recite"))
            .unwrap()
            .contains("Applied prose")
    );
    let (_, model) = writer.session.parts_mut().unwrap();
    model.discard();
    model.undo().unwrap();
    assert!(model.document().source().contains("Hello"));
}
#[test]
fn failed_context_refresh_keeps_the_saved_prefix_and_can_be_retried() {
    let (directory, mut writer, mut jobs) = fixture();
    let manifest = directory.path().join("recite.project.toml");
    let candidate = fs::read_to_string(&manifest)
        .unwrap()
        .replace("content_set='trial'", "content_set='changed'");
    let artifact = directory.path().join("draft.toml");
    fs::write(&artifact, &candidate).unwrap();
    import(&mut writer, &mut jobs, artifact);
    writer.apply_project_settings();
    writer.accept_tool(jobs.try_recv().unwrap().execute());
    assert_eq!(writer.project_settings().unwrap().baseline(), candidate);
    fs::write(&manifest, "invalid TOML [").unwrap();
    super::super::tests::finish(&mut writer, &mut jobs);
    assert!(
        writer
            .status
            .contains("Project settings saved; context refresh failed"),
        "{}",
        writer.status
    );
    assert!(
        writer
            .session
            .project()
            .unwrap()
            .files()
            .configuration_refresh_required()
    );
    fs::write(&manifest, &candidate).unwrap();
    writer.refresh_project_context();
    super::super::tests::finish(&mut writer, &mut jobs);
    assert!(
        !writer
            .session
            .project()
            .unwrap()
            .files()
            .configuration_refresh_required()
    );
}
#[test]
fn stale_import_conflict_and_worker_failure_preserve_the_review_without_saving_manuscript() {
    let (directory, mut writer, mut jobs) = fixture();
    writer.project_settings_action(SettingsAction::ExportDraft);
    super::super::tests::finish(&mut writer, &mut jobs);
    let artifact = writer.workspaces.settings.artifact().unwrap().to_owned();
    let manifest = directory.path().join("recite.project.toml");
    let original = fs::read_to_string(&manifest).unwrap();
    fs::write(&artifact, original.replace("trial", "candidate")).unwrap();
    import(&mut writer, &mut jobs, artifact);
    fs::write(&manifest, original.replace("trial", "external")).unwrap();
    writer.apply_project_settings();
    super::super::tests::finish(&mut writer, &mut jobs);
    assert!(
        writer.status.contains("need confirmation"),
        "{}",
        writer.status
    );
    assert!(
        writer
            .project_settings()
            .unwrap()
            .reviewed_source()
            .is_some()
    );
    assert!(!writer.session.project().unwrap().files().saving);
    writer.workspaces.tools.sender = None;
    writer.project_settings_action(SettingsAction::Reload);
    assert!(!writer.project_jobs_busy());
    assert!(!writer.session.project().unwrap().files().saving);
    assert!(
        writer
            .project_settings()
            .unwrap()
            .reviewed_source()
            .is_some()
    );
}
