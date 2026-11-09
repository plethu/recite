use super::*;
use recite_writer_model::View;
use recite_writer_session::{FileError, ProjectSession};
#[path = "tests/keyed/tests.rs"]
mod keyed;
#[path = "tests/navigation/tests.rs"]
mod navigation;
pub(crate) mod support;
use std::fs;
use support::{click, labels, open, project};

#[test]
fn examples_preserve_drafts_and_offer_explicit_temporary_close_without_file_actions() {
    let mut writer = Writer::from_session(session::Session::examples().unwrap(), None);
    click(&mut writer, &["last_tram_linear.recite"]).unwrap();
    writer.change(|model| {
        model.set_draft("Temporary draft.".into());
        Ok(())
    });
    click(
        &mut writer,
        &["relay_hub.recite", "last_tram_linear.recite"],
    )
    .unwrap();
    assert_eq!(writer.session.model().draft(), "Temporary draft.");
    let names = labels(&mut writer).unwrap();
    assert!(
        !names
            .iter()
            .any(|name| matches!(name.as_str(), "Save file" | "Save all" | "Open in editor"))
    );
    writer.request_close();
    let names = labels(&mut writer).unwrap();
    assert!(
        names
            .iter()
            .any(|name| name == "Close and discard example changes")
    );
    assert!(!names.iter().any(|name| name == "Close and keep recovery"));
    click(&mut writer, &["Keep editing"]).unwrap();
    assert_eq!(writer.session.model().draft(), "Temporary draft.");
}

#[test]
fn unsaved_close_requires_a_decision_and_keeps_a_reopenable_draft() {
    let directory = project(recite_writer_model::FIXTURE).unwrap();
    let mut writer = open(&directory).unwrap();
    writer.change(|model| {
        model.set_draft("Keep this draft.".into());
        Ok(())
    });
    let selected = writer.session.model().view().clone();
    writer.native_field_focus(Some(crate::attention::EditingTarget::passage(
        &writer,
        "missing".into(),
    )));
    assert_eq!(writer.session.model().view(), &selected);
    writer.request_close();
    assert!(writer.phase() == Phase::ConfirmClose);
    assert!(writer.keep_running());
    writer.close_with_recovery();
    assert!(!writer.keep_running());
    drop(writer);
    let mut writer = open(&directory).unwrap();
    assert_eq!(writer.session.model().draft(), "Keep this draft.");
    support::save(&mut writer).unwrap();
    assert!(!writer.session.dirty());
    writer.request_close();
    assert!(writer.phase() == Phase::ConfirmExit);
    click(&mut writer, &["Close Writer"]).unwrap();
    assert!(!writer.keep_running());
    assert!(
        fs::read_to_string(directory.path().join("scene.recite"))
            .unwrap()
            .contains("Keep this draft.")
    );
}

#[test]
fn save_conflict_stays_open_and_reload_preserves_the_local_work() {
    let directory = project(recite_writer_model::FIXTURE).unwrap();
    let mut writer = open(&directory).unwrap();
    writer.change(|model| {
        model.set_draft("My work.".into());
        Ok(())
    });
    fs::write(
        directory.path().join("scene.recite"),
        ":: scene default\n->\n",
    )
    .unwrap();
    support::save(&mut writer).unwrap();
    assert!(writer.status.contains("changed on disk"));
    assert!(writer.keep_running());
    click(&mut writer, &["Project", "Reload (keep recovery copy)"]).unwrap();
    assert_eq!(writer.session.model().view(), &View::Source);
    assert!(writer.status.contains("previous work preserved"));
    click(&mut writer, &["Project", "Reload (keep recovery copy)"]).unwrap();
    assert!(writer.status.contains("previous work preserved"));
    fs::remove_file(directory.path().join("recite.project.toml")).unwrap();
    click(&mut writer, &["Project", "Reload (keep recovery copy)"]).unwrap();
    assert_ne!(writer.status, "Reloaded from disk");
}

#[test]
fn manuscript_and_close_decisions_have_native_accessible_names() {
    let directory = project(recite_writer_model::FIXTURE).unwrap();
    let mut writer = open(&directory).unwrap();
    assert!(
        labels(&mut writer)
            .unwrap()
            .iter()
            .any(|label| label == "Manuscript")
    );
    writer.change(|model| {
        model.set_draft("Draft".into());
        Ok(())
    });
    writer.request_close();
    let labels = labels(&mut writer).unwrap();
    for name in [
        "Keep editing",
        "Save all and close",
        "Close and keep recovery",
    ] {
        assert!(labels.iter().any(|label| label == name), "missing {name}");
    }
}

#[test]
fn malformed_source_offers_external_repair_without_an_editable_manuscript() {
    let directory = project(":: scene default\n->\n").unwrap();
    let mut writer = open(&directory).unwrap();
    let labels = labels(&mut writer).unwrap();
    assert!(!labels.iter().any(|label| label == "Manuscript"));
    assert!(labels.iter().any(|label| label == "Open in editor"));
    assert!(
        labels
            .iter()
            .any(|label| label == "Source editing in your editor")
    );
}

#[test]
fn native_view_actions_apply_discard_undo_and_guard_save() {
    let directory = project(recite_writer_model::FIXTURE).unwrap();
    let mut writer = open(&directory).unwrap();
    let original = writer.session.model().draft().to_owned();
    writer.change(|model| {
        model.set_draft("Accepted.".into());
        Ok(())
    });
    click(&mut writer, &["Project", "Apply draft", "Undo applied"]).unwrap();
    assert_eq!(writer.session.model().draft(), original);
    click(&mut writer, &["Project", "Redo applied"]).unwrap();
    assert_eq!(writer.session.model().draft(), "Accepted.");
    writer.change(|model| {
        model.set_draft("Discard this.".into());
        Ok(())
    });
    click(
        &mut writer,
        &[
            "Project",
            "Discard draft",
            "Project",
            "Close",
            "Keep editing",
        ],
    )
    .unwrap();
    assert!(writer.phase() == Phase::Writing);
    assert_eq!(writer.session.model().draft(), "Accepted.");
    click(&mut writer, &["Project", "Close", "Save all and close"]).unwrap();
    assert!(!writer.keep_running());
    assert!(
        fs::read_to_string(directory.path().join("scene.recite"))
            .unwrap()
            .contains("Accepted.")
    );
}

#[test]
fn native_driver_keeps_the_document_lease_and_uses_the_selected_window_identity() {
    let directory = project(recite_writer_model::FIXTURE).unwrap();
    let writer = open(&directory).unwrap();
    let app = app::App::with_writer(writer);
    let window_id = app.window_id();
    let (driver, mut windows) = app::application(app).into_driver_and_windows(Err);
    assert_eq!(windows.len(), 1);
    let window = windows.pop().unwrap();
    assert_eq!(window.id, window_id);
    assert!(matches!(
        ProjectSession::open(directory.path(), None),
        Err(FileError::RecoveryInUse)
    ));
    drop(driver);
    assert!(open(&directory).is_ok());
}

#[test]
fn ordinary_exit_preference_does_not_weaken_unsaved_work_protection() {
    let directory = project(recite_writer_model::FIXTURE).unwrap();
    let mut writer = open(&directory).unwrap();
    writer.request_close();
    assert!(writer.phase() == Phase::ConfirmExit);
    assert!(
        labels(&mut writer)
            .unwrap()
            .iter()
            .any(|label| label == "Close Writer")
    );
    click(&mut writer, &["Keep editing"]).unwrap();
    writer.config.writer.confirm_exit = false;
    writer.change(|model| {
        model.set_draft("Still protected.".into());
        Ok(())
    });
    writer.request_close();
    assert!(writer.phase() == Phase::ConfirmClose);
    click(&mut writer, &["Keep editing", "Project", "Discard draft"]).unwrap();
    writer.session.project_mut().unwrap().checkpoint().unwrap();
    writer.request_close();
    assert!(!writer.keep_running());
    assert!(
        !fs::read_to_string(directory.path().join("scene.recite"))
            .unwrap()
            .contains("Still protected.")
    );
}
