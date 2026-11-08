use super::*;
use crate::{test_support as support, workspaces::tests::ToolDriver};

const SOURCE: &str = ":: start default\n-> old\n:: old\n-> END\n";
fn fixture() -> (tempfile::TempDir, Writer, ToolDriver) {
    let directory = support::project(SOURCE).unwrap();
    let mut writer = support::open(&directory).unwrap();
    let driver = ToolDriver::new(&mut writer);
    writer.workspaces.screen = Screen::Rename;
    writer.change_rename_block("old".into());
    writer.change_rename_name("new".into());
    (directory, writer, driver)
}
fn request(writer: &mut Writer) -> BeatChoice {
    writer.choose_rename_beat();
    let Some(Request::Rename(request)) = writer.pending_choice.take() else {
        panic!("rename choice")
    };
    request
}

#[test]
fn choosing_changes_only_the_proposal_and_cancel_or_same_beat_retains_the_review() {
    let (_directory, mut writer, mut driver) = fixture();
    writer.review_project_rename();
    driver.finish(&mut writer);
    assert!(
        writer
            .session
            .project()
            .unwrap()
            .files()
            .rename_review
            .is_some()
    );
    let before = writer.session.model().recovery();
    let cancelled = request(&mut writer);
    assert_eq!(
        cancelled
            .choices
            .iter()
            .map(|choice| choice.value.as_str())
            .collect::<Vec<_>>(),
        ["start", "old"]
    );
    drop(cancelled);
    assert!(
        writer
            .session
            .project()
            .unwrap()
            .files()
            .rename_review
            .is_some()
    );
    request(&mut writer).apply(&mut writer, "old".into());
    assert!(
        writer
            .session
            .project()
            .unwrap()
            .files()
            .rename_review
            .is_some()
    );
    request(&mut writer).apply(&mut writer, "start".into());
    assert!(
        writer
            .session
            .project()
            .unwrap()
            .files()
            .rename_review
            .is_none()
    );
    assert_eq!(writer.workspaces.rename.block, "start");
    assert_eq!(writer.workspaces.rename.name, "new");
    assert_eq!(writer.session.model().recovery(), before);
    assert!(!writer.navigation.busy() && !writer.project_jobs_busy());
}

#[test]
fn stale_hidden_or_undeclared_beat_choices_cannot_replace_a_rename_proposal() {
    for change in 0..5 {
        let (_directory, mut writer, _driver) = fixture();
        let choice = request(&mut writer);
        match change {
            0 => writer.change_rename_name("newer".into()),
            1 => writer.workspaces.screen = Screen::Write,
            2 => writer.change(|model| {
                model.select(recite_writer_model::View::Source)?;
                model.set_draft(format!("{SOURCE}\n# newer source\n"));
                model.apply()
            }),
            3 => writer.phase = Phase::ConfirmClose,
            _ => {}
        }
        let before = writer.session.model().recovery();
        choice.apply(
            &mut writer,
            if change == 4 { "not_captured" } else { "start" }.into(),
        );
        assert_eq!(writer.workspaces.rename.block, "old");
        assert_eq!(writer.session.model().recovery(), before);
        assert!(!writer.navigation.busy() && !writer.project_jobs_busy());
    }
}
