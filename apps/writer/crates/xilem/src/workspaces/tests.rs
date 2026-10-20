pub(crate) use super::tools::tests::Driver as ToolDriver;
use super::*;
use crate::tests::support;
use std::fs;
use tokio::sync::mpsc;
#[path = "visual/tests.rs"]
mod visual;

fn finish_build(writer: &mut Writer) {
    for _ in 0..200 {
        writer.poll_project_jobs();
        if !writer.project_jobs_busy() {
            return;
        }
        std::thread::sleep(std::time::Duration::from_millis(10));
    }
    panic!("Build did not retire");
}

pub(super) fn fixture() -> Result<tempfile::TempDir, Box<dyn std::error::Error>> {
    let directory =
        support::project(":: start default\n> line@11111111111111111111\n  Hello.\n-> END\n")?;
    fs::write(
        directory.path().join("recite.project.toml"),
        "format_version=1\n[project]\ncontent_set='trial'\nversion='1'\nschema='schema.json'\n[[scenes]]\nid='start'\nasset='build/start.recitec'\nblock='start'\nparticipants=['mara']\n",
    )?;
    fs::write(
        directory.path().join("schema.json"),
        r#"{"schema_version":1,"speakers":{"mara":{"display_name":"Mara"}}}"#,
    )?;
    Ok(directory)
}
#[test]
fn project_workspaces_open_on_worker_without_applying_or_discarding_manuscript_drafts()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = fixture()?;
    let mut writer = support::open(&directory)?;
    let runtime = tokio::runtime::Runtime::new()?;
    let (sender, mut jobs) = mpsc::unbounded_channel();
    writer.navigation.sender = Some(sender);
    writer.navigation.runtime = Some(runtime.handle().clone());
    writer.change(|model| {
        model.set_draft("Unapplied prose.".into());
        Ok(())
    });
    writer.open_project_workspace(Open::Declarations);
    assert!(writer.navigation.busy());
    assert!(
        writer
            .session
            .project()
            .unwrap()
            .files()
            .declarations
            .is_none()
    );
    support::finish_navigation(&mut writer, &mut jobs)?;
    assert!(writer.workspaces.screen == Screen::Declarations);
    assert_eq!(writer.session.model().draft(), "Unapplied prose.");
    assert_eq!(
        writer
            .session
            .project()
            .unwrap()
            .files()
            .declarations
            .as_ref()
            .unwrap()
            .last_valid_schema()
            .speakers
            .len(),
        1
    );
    writer.open_project_workspace(Open::Build);
    support::finish_navigation(&mut writer, &mut jobs)?;
    assert!(
        writer.workspaces.screen == Screen::Build,
        "{}",
        writer.status
    );
    assert_eq!(
        writer
            .session
            .project()
            .unwrap()
            .files()
            .builds
            .targets
            .len(),
        1
    );
    assert_eq!(writer.session.model().draft(), "Unapplied prose.");
    Ok(())
}

#[test]
fn hidden_build_completion_and_cancel_retirement_keep_close_and_replacement_guarded()
-> Result<(), Box<dyn std::error::Error>> {
    for cancel in [false, true] {
        let directory = fixture()?;
        let mut writer = support::open(&directory)?;
        writer.build_saved();
        assert!(writer.project_jobs_busy());
        writer.return_to_writing();
        if cancel {
            writer.cancel_build();
        }
        writer.request_close();
        assert!(writer.phase == Phase::Writing);
        assert!(
            !writer
                .session
                .project()
                .unwrap()
                .files()
                .can_leave(writer.session.model())
        );
        finish_build(&mut writer);
        assert!(!writer.project_jobs_busy());
        let result = writer
            .session
            .project()
            .unwrap()
            .files()
            .builds
            .result
            .as_ref()
            .unwrap();
        assert!(result.is_success(), "{}", result.message());
        assert!(
            writer
                .session
                .project()
                .unwrap()
                .files()
                .can_leave(writer.session.model())
        );
        writer.request_close();
        assert!(writer.phase == Phase::ConfirmExit);
    }
    Ok(())
}

#[test]
fn returning_to_writing_cancels_a_pending_project_screen_without_adopting_its_owner()
-> Result<(), Box<dyn std::error::Error>> {
    let directory = fixture()?;
    let mut writer = support::open(&directory)?;
    let runtime = tokio::runtime::Runtime::new()?;
    let (sender, mut jobs) = mpsc::unbounded_channel();
    writer.navigation.sender = Some(sender);
    writer.navigation.runtime = Some(runtime.handle().clone());
    writer.open_project_workspace(Open::Declarations);
    let completed = jobs.try_recv()?.execute();
    writer.return_to_writing();
    if let Some(retired) = writer.accept_navigation(completed) {
        retired.retire()?;
    }
    assert!(writer.workspaces.screen == Screen::Write);
    assert!(
        writer
            .session
            .project()
            .unwrap()
            .files()
            .declarations
            .is_none()
    );
    assert!(!writer.navigation.busy());
    Ok(())
}

#[test]
fn save_all_build_uses_shared_save_ordering_and_save_conflicts_prevent_build()
-> Result<(), Box<dyn std::error::Error>> {
    for conflict in [false, true] {
        let directory = fixture()?;
        let mut writer = support::open(&directory)?;
        let (sender, mut work) = mpsc::unbounded_channel();
        writer.work.sender = Some(sender);
        writer.change(|model| {
            model.set_draft("Ready to build.".into());
            Ok(())
        });
        if conflict {
            fs::write(directory.path().join("scene.recite"), "external")?;
        }
        writer.save_and_build();
        support::finish_work(&mut writer, &mut work)?;
        assert_eq!(writer.project_jobs_busy(), !conflict);
        if conflict {
            assert_eq!(
                fs::read_to_string(directory.path().join("scene.recite"))?,
                "external"
            );
            assert!(!directory.path().join("build/start.recitec").exists());
        } else {
            assert!(
                fs::read_to_string(directory.path().join("scene.recite"))?
                    .contains("Ready to build.")
            );
            finish_build(&mut writer);
            assert!(directory.path().join("build/start.recitec").is_file());
        }
    }
    Ok(())
}

#[test]
fn report_pages_bound_a_single_long_unicode_line_without_dropping_text() {
    let report = "駅🚉".repeat(10_000);
    let pages = super::text::pages(&report);
    assert_eq!(pages.len(), 20);
    assert!(pages.iter().all(|page| page.chars().count() <= 1_000));
    assert_eq!(pages.concat(), report);
}
