use crate::workspaces::tests::{finish_build, fixture};
use crate::{test_support as support, workspaces::Screen};
use std::fs;

#[test]
fn the_build_command_builds_at_once_and_its_result_waits_on_a_slip_away_from_the_page() {
    let directory = fixture().unwrap();
    let mut writer = support::open(&directory).unwrap();
    writer.build_scenes();
    assert!(writer.project_jobs_busy());
    assert!(writer.workspaces.screen == Screen::Write);
    assert!(writer.build_result().is_none());
    finish_build(&mut writer);
    assert!(directory.path().join("build/start.recitec").is_file());
    let result = writer.build_result().unwrap();
    assert!(result.succeeded);
    assert_eq!(result.headline, "Built build/start.recitec");
    assert!(result.detail.is_none());
    assert_eq!(writer.status, result.headline);
    writer.dismiss_build_result();
    assert!(writer.build_result().is_none());

    // A failure offers its report; opening the page takes the slip away.
    fs::write(directory.path().join("schema.json"), "not json").unwrap();
    writer.build_scenes();
    finish_build(&mut writer);
    let failure = writer.build_result().unwrap();
    assert!(!failure.succeeded);
    assert_eq!(failure.headline, "Build did not complete");
    assert!(failure.detail.unwrap().contains("schema"));
    writer.open_build_page();
    assert!(writer.build_result().is_none());
}
