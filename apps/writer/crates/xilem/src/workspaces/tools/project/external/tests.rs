use super::*;
use crate::workspaces::tools::{ExternalAction, tests::finish};
use recite_writer_model::View;
use recite_writer_session::project::ExternalDecision;

fn compare(writer: &mut Writer, jobs: &mut mpsc::UnboundedReceiver<Job>, root: &std::path::Path) {
    writer
        .session
        .edit(|model| {
            model
                .select(View::Passage("11111111111111111111".into()))
                .unwrap();
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
        &View::Passage("11111111111111111111".into())
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
        writer.session.model().view(),
        &View::Passage("11111111111111111111".into())
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
