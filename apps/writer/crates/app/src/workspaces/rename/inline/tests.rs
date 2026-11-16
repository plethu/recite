use super::*;
use crate::{test_support as support, workspaces::tests::ToolDriver};
use std::fs;
use tokio::sync::mpsc;

const SOURCE: &str = ":: start default\n-> old\n:: old\n-> END\n";
const OTHER: &str = ":: elsewhere\n-> scene.recite::old\n";

/// A project whose beat `old` is selected, its references in two files.
fn selected_old() -> (tempfile::TempDir, Writer, tokio::runtime::Runtime) {
    let directory = support::project(SOURCE).unwrap();
    fs::write(directory.path().join("zz-other.recite"), OTHER).unwrap();
    let mut writer = support::open(&directory).unwrap();
    let (sender, mut jobs) = mpsc::unbounded_channel();
    writer.navigation.sender = Some(sender);
    let runtime = tokio::runtime::Runtime::new().unwrap();
    writer.navigation.runtime = Some(runtime.handle().clone());
    writer.inspect("old".into());
    support::finish_navigation(&mut writer, &mut jobs).unwrap();
    (directory, writer, runtime)
}

#[test]
fn enter_reviews_the_rename_and_the_slip_stages_it_as_one_undo_step() {
    let (directory, mut writer, _runtime) = selected_old();
    let mut tools = ToolDriver::new(&mut writer);
    writer.rename_beat();
    let inline = writer.inline_rename().unwrap();
    assert_eq!((inline.beat.as_str(), inline.name.as_str()), ("old", "old"));
    assert_eq!(inline.stage, Stage::Editing);
    writer.edit_inline_rename("new".into());
    writer.commit_inline_rename();
    assert_eq!(writer.inline_rename().unwrap().stage, Stage::Reviewing);
    // Typing while the review runs changes nothing.
    writer.edit_inline_rename("other".into());
    tools.finish(&mut writer);
    assert_eq!(writer.inline_rename().unwrap().stage, Stage::Reviewed);
    let consequence = writer.rename_consequence().unwrap();
    assert_eq!(
        consequence,
        Consequence {
            references: 3,
            files: 2
        }
    );
    assert_eq!(
        consequence.sentence("old", "new"),
        "Renames old to new in 3 references across 2 files"
    );
    writer.confirm_inline_rename();
    assert_eq!(writer.inline_rename().unwrap().stage, Stage::Applying);
    tools.finish(&mut writer);
    assert!(writer.inline_rename().is_none());
    assert!(writer.workspaces.screen == Screen::Write);
    assert!(writer.status.starts_with("Renamed old to new."));
    assert!(writer.offers_rename_undo());
    assert!(
        writer
            .session
            .model()
            .document()
            .source()
            .contains(":: new")
    );
    // Nothing is written until Save all, and Undo restores the whole group.
    assert!(
        fs::read_to_string(directory.path().join("zz-other.recite"))
            .unwrap()
            .contains("::old")
    );
    writer.undo_project_rename();
    tools.finish(&mut writer);
    assert!(
        writer
            .session
            .model()
            .document()
            .source()
            .contains(":: old")
    );
    assert!(!writer.offers_rename_undo());
}

#[test]
fn cancel_and_an_unchanged_name_leave_the_project_untouched() {
    let (_directory, mut writer, _runtime) = selected_old();
    let mut tools = ToolDriver::new(&mut writer);
    writer.begin_inline_rename();
    writer.commit_inline_rename();
    assert!(
        writer.inline_rename().is_none(),
        "an unchanged name ends it"
    );
    writer.begin_inline_rename();
    writer.edit_inline_rename("new".into());
    writer.commit_inline_rename();
    tools.finish(&mut writer);
    writer.cancel_inline_rename();
    assert!(writer.inline_rename().is_none());
    assert!(
        writer
            .session
            .project()
            .unwrap()
            .files()
            .rename_review
            .is_none()
    );
    assert!(
        writer
            .session
            .model()
            .document()
            .source()
            .contains(":: old")
    );
}

#[test]
fn a_refused_rename_and_review_all_changes_open_the_rename_page() {
    let (_directory, mut writer, _runtime) = selected_old();
    let mut tools = ToolDriver::new(&mut writer);
    writer.begin_inline_rename();
    writer.edit_inline_rename("start".into());
    writer.commit_inline_rename();
    tools.finish(&mut writer);
    assert!(writer.inline_rename().is_none());
    assert!(matches!(
        writer.workspaces.rename.report,
        Some(Report::Problem(_))
    ));
    assert_eq!(
        (
            writer.workspaces.rename.block.as_str(),
            writer.workspaces.rename.name.as_str()
        ),
        ("old", "start")
    );
    writer.show_workspace(crate::map::Workspace::Script);
    writer.begin_inline_rename();
    writer.edit_inline_rename("new".into());
    writer.commit_inline_rename();
    tools.finish(&mut writer);
    writer.review_inline_rename();
    assert!(writer.inline_rename().is_none());
    assert!(!writer.workspaces.rename.changes.is_empty());
}
