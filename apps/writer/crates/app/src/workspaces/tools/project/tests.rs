// Fixtures here are shared with frontend tests through the `test-support` feature.
#![allow(unused_imports, dead_code, clippy::unwrap_used, clippy::expect_used)]
pub use super::*;
pub use crate::{
    test_support as support,
    workspaces::tools::{Completed as ToolCompleted, Work as ToolWork},
};
pub use std::fs;
pub use tokio::sync::mpsc;
pub const SOURCE: &str = ":: start default\n> @ffee6p8\n  Hello.\n-> old\n\n:: old\n-> END\n";
pub fn fixture() -> (tempfile::TempDir, Writer, mpsc::UnboundedReceiver<Job>) {
    let directory = support::project(SOURCE).unwrap();
    fs::write(
        directory.path().join("other.recite"),
        ":: elsewhere\n-> scene.recite::old\n",
    )
    .unwrap();
    let mut writer = support::open(&directory).unwrap();
    let (sender, jobs) = mpsc::unbounded_channel();
    writer.workspaces.tools.sender = Some(sender);
    (directory, writer, jobs)
}
pub fn review(writer: &mut Writer, jobs: &mut mpsc::UnboundedReceiver<Job>) {
    writer.change_rename_block("old".into());
    writer.change_rename_name("new".into());
    writer.review_project_rename();
    assert!(writer.session.checked_out());
    super::super::tests::finish(writer, jobs);
    assert!(
        writer
            .session
            .project()
            .unwrap()
            .files()
            .rename_review
            .is_some(),
        "{}",
        writer.status
    );
}
#[test]
pub fn review_apply_and_grouped_history_return_the_same_project_with_valid_native_selection() {
    let (directory, mut writer, mut jobs) = fixture();
    let view = writer.session.model().view().clone();
    review(&mut writer, &mut jobs);
    assert_eq!(writer.session.model().document().source(), SOURCE);
    writer.project_action(Action::ApplyRename);
    assert!(writer.session.read_only());
    writer.change(|_| panic!("queued field edit must not run"));
    super::super::tests::finish(&mut writer, &mut jobs);
    assert!(!writer.session.read_only(), "{}", writer.status);
    assert_eq!(writer.session.model().view(), &view);
    assert!(
        writer
            .session
            .model()
            .document()
            .source()
            .contains(":: new")
    );
    assert_eq!(
        fs::read_to_string(directory.path().join("scene.recite")).unwrap(),
        SOURCE
    );
    assert!(writer.has_project_history(false));
    writer.undo_redo(false);
    super::super::tests::finish(&mut writer, &mut jobs);
    assert_eq!(writer.session.model().document().source(), SOURCE);
    assert!(writer.has_project_history(true));
    writer.undo_redo(true);
    super::super::tests::finish(&mut writer, &mut jobs);
    assert!(
        writer
            .session
            .model()
            .document()
            .source()
            .contains(":: new")
    );
    assert_eq!(writer.session.model().view(), &view);
}
#[test]
pub fn cancelled_or_undispatched_action_returns_the_actual_lease_owner() {
    let (directory, mut writer, _jobs) = fixture();
    writer.workspaces.tools.sender = None;
    writer.project_action(Action::ReviewRename {
        block: "old".into(),
        name: "new".into(),
    });
    assert!(writer.session.project().is_some());
    assert!(!writer.session.read_only());
    let (sender, next) = mpsc::unbounded_channel();
    writer.workspaces.tools.sender = Some(sender);
    let mut jobs = next;
    writer.project_action(Action::ReviewRename {
        block: "old".into(),
        name: "new".into(),
    });
    writer.cancel_project_action();
    super::super::tests::finish(&mut writer, &mut jobs);
    assert!(!writer.session.read_only());
    assert!(
        writer
            .session
            .project()
            .unwrap()
            .files()
            .rename_review
            .is_none()
    );
    assert!(matches!(
        ProjectSession::open(directory.path(), None),
        Err(FileError::RecoveryInUse)
    ));
}
#[test]
pub fn cancellation_after_committed_rename_cannot_discard_the_returned_project() {
    let (_directory, mut writer, mut jobs) = fixture();
    review(&mut writer, &mut jobs);
    writer.project_action(Action::ApplyRename);
    let job = jobs.try_recv().unwrap();
    let ToolWork::Project(work) = job.work else {
        panic!("exclusive work");
    };
    let cancel = work.checkout.ticket.clone();
    let completed = work.execute_with(|action, project, control| {
        let result = action.perform(project, control);
        cancel.cancel();
        result
    });
    let retired = writer.accept_tool(ToolCompleted {
        request: job.request,
        result: Ok(Value::Project(Box::new(completed))),
    });
    drop(retired);
    assert!(!writer.session.read_only());
    assert!(
        writer
            .session
            .model()
            .document()
            .source()
            .contains(":: new")
    );
}
#[test]
pub fn failed_recovery_after_commit_freezes_edits_and_retry_releases_the_project() {
    let (directory, mut writer, mut jobs) = fixture();
    review(&mut writer, &mut jobs);
    let recovery = directory
        .path()
        .join("scene.recite.recite-editor-recovery.json");
    fs::create_dir(&recovery).unwrap();
    writer.project_action(Action::ApplyRename);
    super::super::tests::finish(&mut writer, &mut jobs);
    assert!(writer.session.read_only(), "{}", writer.status);
    assert!(!writer.session.checked_out());
    assert!(
        writer
            .session
            .model()
            .document()
            .source()
            .contains(":: new")
    );
    assert!(writer.session.edit(|_| panic!("recovery pending")).is_err());
    assert!(
        writer
            .session
            .project()
            .unwrap()
            .files()
            .project_edit_recovery_pending()
    );
    fs::remove_dir(&recovery).unwrap();
    writer.project_action(Action::FinishRecovery);
    super::super::tests::finish(&mut writer, &mut jobs);
    assert!(!writer.session.read_only(), "{}", writer.status);
    assert!(
        !writer
            .session
            .project()
            .unwrap()
            .files()
            .project_edit_recovery_pending()
    );
    assert!(
        writer
            .session
            .model()
            .document()
            .source()
            .contains(":: new")
    );
}
#[test]
pub fn caught_panic_returns_the_owner_with_editing_excluded_until_recovery() {
    let (_directory, mut writer, mut jobs) = fixture();
    writer.project_action(Action::ReviewRename {
        block: "old".into(),
        name: "new".into(),
    });
    let job = jobs.try_recv().unwrap();
    let ToolWork::Project(work) = job.work else {
        panic!("exclusive work");
    };
    let completed = work.execute_with(|_, _, _| panic!("injected operation panic"));
    drop(writer.accept_tool(ToolCompleted {
        request: job.request,
        result: Ok(Value::Project(Box::new(completed))),
    }));
    assert!(writer.session.read_only());
    assert!(writer.session.project().is_some());
    assert!(!writer.project_jobs_busy());
    writer.project_action(Action::FinishRecovery);
    assert!(writer.session.read_only());
    assert!(!writer.session.checked_out());
    assert_eq!(
        writer.session.recovery_reason(),
        Some(RecoveryReason::ReopenRequired)
    );
    writer.request_close();
    assert!(writer.phase == Phase::ConfirmClose);
    writer.close_with_recovery();
    assert!(writer.phase == Phase::Closed);
}

#[test]
pub fn changing_the_proposal_retires_the_old_review_before_a_queued_apply() {
    let (_directory, mut writer, mut jobs) = fixture();
    review(&mut writer, &mut jobs);
    writer.change_rename_name("different".into());
    assert!(
        writer
            .session
            .project()
            .unwrap()
            .files()
            .rename_review
            .is_none()
    );
    writer.apply_project_rename();
    super::super::tests::finish(&mut writer, &mut jobs);
    assert_eq!(writer.session.model().document().source(), SOURCE);
    assert!(!writer.has_project_history(false));
}
