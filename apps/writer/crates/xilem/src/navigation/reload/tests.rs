use super::*;
use std::fs;

#[test]
fn reload_adopts_only_on_completion_and_keeps_the_complete_previous_work() {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    writer.change(|model| {
        model.set_draft("Previous field draft.".into());
        Ok(())
    });
    let path = fixture.directory.path().join("scene.recite");
    fs::write(&path, SOURCE.replace("First.", "External edit.")).unwrap();
    writer.reload();
    assert!(writer.navigation.busy());
    assert_eq!(writer.session.model().document().source(), SOURCE);
    let completed = fixture.jobs.try_recv().unwrap().execute();
    assert_eq!(writer.session.model().draft(), "Previous field draft.");
    let retired = writer.accept_navigation(completed).unwrap();
    assert!(
        writer
            .session
            .model()
            .document()
            .source()
            .contains("External edit.")
    );
    retired.retire().unwrap();
    assert!(!writer.session.dirty());
    let archive = fs::read_to_string(writer.recovery_copy.as_ref().unwrap()).unwrap();
    assert!(archive.contains("Previous field draft."));
    assert_eq!(
        fs::read_to_string(path).unwrap(),
        SOURCE.replace("First.", "External edit.")
    );
}

#[test]
fn cancelled_or_stale_reload_keeps_current_work_and_reports_its_completed_archive() {
    for cancel in [false, true] {
        let mut fixture = fixture().unwrap();
        let writer = &mut fixture.writer;
        writer.change(|model| {
            model.set_draft("Captured draft.".into());
            Ok(())
        });
        writer.reload();
        let completed = fixture.jobs.try_recv().unwrap().execute();
        if cancel {
            writer.cancel_navigation();
        } else {
            writer.change(|model| {
                model.set_draft("Newer typing.".into());
                Ok(())
            });
        }
        writer
            .accept_navigation(completed)
            .unwrap()
            .retire()
            .unwrap();
        assert!(!writer.navigation.busy());
        assert_eq!(writer.session.model().document().source(), SOURCE);
        assert_eq!(
            writer.session.model().draft(),
            if cancel {
                "Captured draft."
            } else {
                "Newer typing."
            }
        );
        assert!(writer.recovery_copy.as_ref().unwrap().is_file());
        writer.session.project_mut().unwrap().checkpoint().unwrap();
        assert!(writer.status.contains("current work stays open"));
    }
}

#[test]
fn failed_reload_leaves_draft_live_and_can_be_retried() {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    writer.change(|model| {
        model.set_draft("Keep my draft.".into());
        Ok(())
    });
    let path = fixture.directory.path().join("scene.recite");
    writer.reload();
    fs::remove_file(&path).unwrap();
    finish_navigation(writer, &mut fixture.jobs).unwrap();
    assert_eq!(writer.session.model().draft(), "Keep my draft.");
    assert!(writer.recovery_copy.is_none());
    fs::write(&path, SOURCE).unwrap();
    writer.reload();
    finish_navigation(writer, &mut fixture.jobs).unwrap();
    assert!(!writer.session.dirty());
    assert!(writer.recovery_copy.as_ref().unwrap().is_file());
}

#[test]
fn cleanup_failure_is_attributed_to_the_adopted_document_without_rolling_it_back() {
    let mut fixture = fixture().unwrap();
    let writer = &mut fixture.writer;
    writer.change(|model| {
        model.set_draft("Archived work.".into());
        Ok(())
    });
    writer.session.project_mut().unwrap().checkpoint().unwrap();
    let sidecar = fixture
        .directory
        .path()
        .join("scene.recite.recite-editor-recovery.json");
    fs::remove_file(&sidecar).unwrap();
    fs::create_dir(&sidecar).unwrap();
    fs::write(
        fixture.directory.path().join("scene.recite"),
        SOURCE.replace("First.", "External prose."),
    )
    .unwrap();
    writer.reload();
    let completed = fixture.jobs.try_recv().unwrap().execute();
    let retired = writer.accept_navigation(completed).unwrap();
    let message = retired.retire().unwrap_err();
    assert!(message.contains("Reload accepted for"));
    assert!(message.contains("scene.recite"));
    assert!(message.contains("recovery cleanup failed"));
    assert!(
        writer
            .session
            .model()
            .document()
            .source()
            .contains("External prose.")
    );
    assert!(writer.recovery_copy.as_ref().unwrap().is_file());
    fs::remove_dir(&sidecar).unwrap();
    writer.session.project_mut().unwrap().checkpoint().unwrap();
    assert!(!writer.session.dirty());
}
