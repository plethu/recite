#![allow(unused_imports)]
use super::*;
use crate::{
    attention::{Pane, PaneMotion},
    workspaces::{
        Screen,
        external::Resolution,
        tools::{ExternalAction, tests::finish},
    },
};
use recite_writer_model::View;
use recite_writer_session::project::ExternalDecision;
use std::path::PathBuf;

fn compare(writer: &mut Writer, jobs: &mut mpsc::UnboundedReceiver<Job>, root: &std::path::Path) {
    writer
        .session
        .edit(|model| {
            model.select(View::Passage("ffee6p8".into())).unwrap();
            model.set_draft("Pending manuscript.".into());
            Ok(())
        })
        .unwrap();
    fs::write(
        root.join("scene.recite"),
        SOURCE.replace("Hello.", "External."),
    )
    .unwrap();
    writer.compare_disk_changes();
    assert!(writer.session.checked_out());
    finish(writer, jobs);
    assert_eq!(writer.session.model().draft(), "Pending manuscript.");
    assert_eq!(
        writer.session.model().view(),
        &View::Passage("ffee6p8".into())
    );
}
#[test]
fn native_comparison_keeps_pending_prose_or_adopts_disk_without_writing_source() {
    for decision in [
        ExternalDecision::KeepWriter,
        ExternalDecision::UseInspectedDisk,
    ] {
        let (directory, mut writer, mut jobs) = fixture();
        compare(&mut writer, &mut jobs, directory.path());
        writer.project_action(Action::External(ExternalAction::Resolve(decision)));
        finish(&mut writer, &mut jobs);
        assert_eq!(
            writer
                .workspaces
                .external
                .resolved()
                .map(|resolved| resolved.resolution),
            Some(match decision {
                ExternalDecision::KeepWriter => Resolution::KeepWriter,
                ExternalDecision::UseInspectedDisk => Resolution::UseInspectedDisk,
            })
        );
        assert!(writer.workspaces.external.inspection().is_none());
        assert_eq!(
            writer.attention.pane_request().unwrap().motion(),
            PaneMotion::To(Pane::External)
        );
        assert!(!writer.session.read_only(), "{}", writer.status);
        assert!(writer.recovery_copy.as_ref().unwrap().is_file());
        let model = writer.session.model();
        if decision == ExternalDecision::KeepWriter {
            assert_eq!(model.draft(), "Pending manuscript.");
            assert_eq!(model.document().source(), SOURCE);
        } else {
            assert_eq!(model.draft(), "External.");
            writer.session.edit(|model| model.undo()).unwrap();
            assert_eq!(writer.session.model().document().source(), SOURCE);
        }
        assert!(
            fs::read_to_string(directory.path().join("scene.recite"))
                .unwrap()
                .contains("External.")
        );
    }
}
#[test]
fn merge_handoff_import_retains_selection_and_one_history_step() {
    let (directory, mut writer, mut jobs) = fixture();
    compare(&mut writer, &mut jobs, directory.path());
    writer.export_merge_draft();
    finish(&mut writer, &mut jobs);
    let source = writer
        .session
        .project()
        .unwrap()
        .files()
        .external
        .as_ref()
        .unwrap()
        .artifact()
        .unwrap()
        .source
        .clone();
    assert!(
        fs::read_to_string(&source)
            .unwrap()
            .contains("Pending manuscript.")
    );
    fs::write(&source, SOURCE.replace("Hello.", "Reviewed merge.")).unwrap();
    writer.import_reviewed_merge();
    finish(&mut writer, &mut jobs);
    assert_eq!(writer.session.model().draft(), "Reviewed merge.");
    assert_eq!(
        writer
            .workspaces
            .external
            .resolved()
            .map(|resolved| resolved.resolution),
        Some(Resolution::Merged)
    );
    assert_eq!(
        writer.session.model().view(),
        &View::Passage("ffee6p8".into())
    );
    writer.session.edit(|model| model.undo()).unwrap();
    assert_eq!(writer.session.model().document().source(), SOURCE);
    assert!(
        fs::read_to_string(directory.path().join("scene.recite"))
            .unwrap()
            .contains("External.")
    );
}
#[test]
fn recovery_failure_after_admission_still_returns_the_adopted_model_and_archive() {
    let (directory, mut writer, mut jobs) = fixture();
    compare(&mut writer, &mut jobs, directory.path());
    writer.keep_writer_version();
    let job = jobs.try_recv().unwrap();
    let ToolWork::Project(work) = job.work else {
        panic!("exclusive work");
    };
    let completed = work.execute_with(|action, project, control| {
        let path = directory
            .path()
            .join("scene.recite.recite-editor-recovery.json");
        fs::remove_file(&path).unwrap();
        fs::create_dir(path).unwrap();
        action.perform(project, control)
    });
    drop(writer.accept_tool(ToolCompleted {
        request: job.request,
        result: Ok(Value::Project(Box::new(completed))),
    }));
    assert!(!writer.session.read_only());
    assert_eq!(writer.session.model().draft(), "Pending manuscript.");
    assert!(writer.session.project().unwrap().files().external.is_none());
    assert!(writer.recovery_copy.as_ref().unwrap().is_file());
    assert!(writer.status.contains("adopted"), "{}", writer.status);
    assert_eq!(
        writer
            .workspaces
            .external
            .resolved()
            .map(|resolved| resolved.resolution),
        Some(Resolution::KeepWriter)
    );
}

#[test]
fn an_adopted_result_survives_a_failed_review_projection_and_failed_reinspection() {
    let (directory, mut writer, mut jobs) = fixture();
    compare(&mut writer, &mut jobs, directory.path());
    writer.keep_writer_version();
    let job = jobs.try_recv().unwrap();
    let ToolWork::Project(work) = job.work else {
        panic!("exclusive work")
    };
    let completed =
        work.execute_with_presentation(Action::perform, |_, _| panic!("review projection"));
    assert!(matches!(&completed.outcome, Outcome::Ready(Ok(success))
        if matches!(success.effect, Effect::ExternalResolved(Resolved { resolution: Resolution::KeepWriter, .. }))
            && success.warning.as_ref().is_some_and(|warning| warning.contains("could not be rendered"))));
    drop(writer.accept_tool(ToolCompleted {
        request: job.request,
        result: Ok(Value::Project(Box::new(completed))),
    }));
    assert_eq!(
        writer
            .workspaces
            .external
            .resolved()
            .map(|resolved| resolved.resolution),
        Some(Resolution::KeepWriter)
    );
    assert!(writer.recovery_copy.as_ref().unwrap().is_file());
    assert_eq!(writer.session.model().draft(), "Pending manuscript.");
    let source = directory.path().join("scene.recite");
    fs::remove_file(&source).unwrap();
    writer.compare_disk_changes();
    finish(&mut writer, &mut jobs);
    assert_eq!(
        writer
            .workspaces
            .external
            .resolved()
            .map(|resolved| resolved.resolution),
        Some(Resolution::KeepWriter)
    );
    fs::write(source, SOURCE).unwrap();
    writer.compare_disk_changes();
    finish(&mut writer, &mut jobs);
    assert!(
        writer
            .workspaces
            .external
            .resolved()
            .map(|resolved| resolved.resolution)
            .is_none()
    );
    assert!(writer.workspaces.external.inspection().is_some());
}

#[test]
fn a_failed_comparison_on_another_document_cannot_reuse_an_earlier_resolved_outcome() {
    let (directory, mut writer, mut jobs) = fixture();
    compare(&mut writer, &mut jobs, directory.path());
    writer.use_inspected_disk();
    finish(&mut writer, &mut jobs);
    assert_eq!(
        writer.workspaces.external.resolved().unwrap().path,
        directory.path().join("scene.recite")
    );
    writer.return_to_writing();
    let other = directory.path().join("other.recite");
    writer
        .session
        .project_mut()
        .unwrap()
        .switch(&other)
        .unwrap();
    fs::remove_file(other).unwrap();
    writer.compare_disk_changes();
    finish(&mut writer, &mut jobs);
    assert!(writer.workspaces.external.resolved().is_none());
    assert!(writer.workspaces.external.inspection().is_none());
}

#[test]
fn rejected_adoption_exposes_the_archive_created_before_cancellation() {
    struct AfterArchive<'a> {
        root: &'a std::path::Path,
        control: &'a dyn WorkControl,
    }
    impl WorkControl for AfterArchive<'_> {
        fn checkpoint(&self) -> Result<(), recite_compiler::authoring::Interrupted> {
            self.control.checkpoint()?;
            if fs::read_dir(self.root).unwrap().any(|entry| {
                entry
                    .unwrap()
                    .file_name()
                    .to_string_lossy()
                    .starts_with(".recite-recovered-")
            }) {
                return Err(recite_compiler::authoring::Interrupted);
            }
            Ok(())
        }
    }
    let (directory, mut writer, mut jobs) = fixture();
    compare(&mut writer, &mut jobs, directory.path());
    writer.use_inspected_disk();
    let job = jobs.try_recv().unwrap();
    let ToolWork::Project(work) = job.work else {
        panic!("exclusive work");
    };
    let completed = work.execute_with(|action, project, control| {
        action.perform(
            project,
            &AfterArchive {
                root: directory.path(),
                control,
            },
        )
    });
    drop(writer.accept_tool(ToolCompleted {
        request: job.request,
        result: Ok(Value::Project(Box::new(completed))),
    }));
    assert_eq!(writer.session.model().document().source(), SOURCE);
    assert_eq!(writer.session.model().draft(), "Pending manuscript.");
    let archive = writer.recovery_copy.clone().unwrap();
    assert!(archive.is_file());
    writer.recovery_details = true;
    let labels = support::labels(&mut writer).unwrap();
    assert!(
        labels
            .iter()
            .any(|label| label.contains(&archive.display().to_string()))
    );
}

#[test]
fn completed_disk_decision_does_not_reopen_or_focus_a_workspace_the_author_left() {
    let (directory, mut writer, mut jobs) = fixture();
    compare(&mut writer, &mut jobs, directory.path());
    writer.keep_writer_version();
    writer.return_to_writing();
    let attention = writer.attention.clone();
    finish(&mut writer, &mut jobs);
    assert!(writer.workspaces.screen == Screen::Write);
    assert_eq!(writer.attention.pane_request(), attention.pane_request());
    assert_eq!(
        writer.workspaces.external.resolved().unwrap().resolution,
        Resolution::KeepWriter
    );
    assert_eq!(writer.session.model().draft(), "Pending manuscript.");
}
