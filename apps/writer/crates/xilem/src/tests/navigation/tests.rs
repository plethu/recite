use super::*;

#[test]
fn inactive_draft_guards_window_closing_and_save_close_covers_every_open_file() {
    let directory = project(recite_writer_model::FIXTURE).unwrap();
    let other = directory.path().join("other.recite");
    fs::write(
        &other,
        ":: other\n> other@aaaaaaaaaaaaaaaaaaaa\n  Other paragraph.\n-> END\n",
    )
    .unwrap();
    let mut writer = open(&directory).unwrap();
    writer.change(|model| {
        model.set_draft("Keep my first draft.".into());
        Ok(())
    });
    click(&mut writer, &["other.recite"]).unwrap();
    assert_eq!(writer.session.project().unwrap().current().path(), other);
    assert!(!writer.session.project().unwrap().current().dirty());
    click(&mut writer, &["Close"]).unwrap();
    assert!(writer.phase == Phase::ConfirmClose);
    click(&mut writer, &["Keep editing"]).unwrap();
    assert!(writer.phase == Phase::Writing);
    click(&mut writer, &["Close", "Save all and close"]).unwrap();
    assert!(!writer.keep_running());
    assert!(!writer.session.dirty());
    assert!(
        fs::read_to_string(directory.path().join("scene.recite"))
            .unwrap()
            .contains("Keep my first draft.")
    );
}

#[test]
fn failed_inactive_save_keeps_the_window_open_and_displays_the_conflicting_file() {
    let directory = project(recite_writer_model::FIXTURE).unwrap();
    let other = directory.path().join("other.recite");
    fs::write(&other, ":: other\n-> END\n").unwrap();
    let mut writer = open(&directory).unwrap();
    writer.change(|model| {
        model.set_draft("Local first draft.".into());
        Ok(())
    });
    click(&mut writer, &["other.recite"]).unwrap();
    fs::write(
        directory.path().join("scene.recite"),
        ":: external\n-> END\n",
    )
    .unwrap();
    click(&mut writer, &["Close", "Save all and close"]).unwrap();
    assert!(writer.keep_running());
    assert!(writer.phase == Phase::ConfirmClose);
    assert!(writer.status.contains("scene.recite"));
    assert!(writer.status.contains("changed on disk"));
    click(&mut writer, &["Keep editing"]).unwrap();
    writer.switch(&directory.path().join("missing.recite"));
    assert_eq!(writer.session.project().unwrap().current().path(), other);
}

#[test]
fn file_pages_bound_native_controls_without_opening_unvisited_documents() {
    let directory = project(recite_writer_model::FIXTURE).unwrap();
    for index in 0..12 {
        fs::write(
            directory.path().join(format!("file_{index:02}.recite")),
            format!(":: file_{index}\n-> END\n"),
        )
        .unwrap();
    }
    let mut writer = open(&directory).unwrap();
    let before = labels(&mut writer).unwrap();
    assert!(before.iter().any(|label| label == "file_00.recite"));
    assert!(!before.iter().any(|label| label == "file_04.recite"));
    click(&mut writer, &["Next files"]).unwrap();
    let after = labels(&mut writer).unwrap();
    assert!(after.iter().any(|label| label == "file_04.recite"));
    assert!(!after.iter().any(|label| label == "file_00.recite"));
    click(&mut writer, &["Previous files"]).unwrap();
    assert_eq!(writer.file_page, 0);
    assert_eq!(writer.session.project().unwrap().sessions().count(), 1);
}
