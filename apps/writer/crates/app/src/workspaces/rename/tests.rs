use super::*;
use crate::{test_support as support, workspaces::tests::ToolDriver};
use std::fs;
use tokio::sync::mpsc;

const SOURCE: &str = ":: start default\n-> old\n:: old\n-> END\n";
const OTHER: &str = ":: elsewhere\n-> scene.recite::old\n";

fn staged() -> (tempfile::TempDir, Writer) {
    let directory = support::project(SOURCE).unwrap();
    fs::write(directory.path().join("zz-other.recite"), OTHER).unwrap();
    let mut writer = support::open(&directory).unwrap();
    let mut tools = ToolDriver::new(&mut writer);
    writer.change_rename_block("old".into());
    writer.change_rename_name("new".into());
    writer.review_project_rename();
    tools.finish(&mut writer);
    writer.apply_project_rename();
    tools.finish(&mut writer);
    assert!(matches!(
        writer.workspaces.rename.report,
        Some(Report::Information(_))
    ));
    (directory, writer)
}

#[test]
fn partial_save_keeps_staging_guidance_and_complete_save_retires_it_without_losing_undo() {
    let (directory, mut writer) = staged();
    let (sender, mut jobs) = mpsc::unbounded_channel();
    writer.work.sender = Some(sender);
    let other = directory.path().join("zz-other.recite");
    fs::write(&other, ":: external\n-> END\n").unwrap();
    writer.save_all();
    support::finish_work(&mut writer, &mut jobs).unwrap();
    assert!(
        fs::read_to_string(directory.path().join("scene.recite"))
            .unwrap()
            .contains(":: new")
    );
    assert!(writer.workspaces.rename.report.is_some());
    assert!(writer.has_project_history(false));
    fs::write(&other, OTHER).unwrap();
    writer.save_all();
    support::finish_work(&mut writer, &mut jobs).unwrap();
    assert!(
        fs::read_to_string(&other)
            .unwrap()
            .contains("scene.recite::new")
    );
    assert!(writer.workspaces.rename.report.is_none());
    assert!(writer.has_project_history(false));
}

#[test]
fn successful_save_preserves_a_problem_report() {
    let (_directory, mut writer) = staged();
    writer
        .workspaces
        .rename
        .report(Some("Recovery needs inspection."));
    let (sender, mut jobs) = mpsc::unbounded_channel();
    writer.work.sender = Some(sender);
    writer.save_all();
    support::finish_work(&mut writer, &mut jobs).unwrap();
    assert!(matches!(
        writer.workspaces.rename.report,
        Some(Report::Problem(_))
    ));
}
