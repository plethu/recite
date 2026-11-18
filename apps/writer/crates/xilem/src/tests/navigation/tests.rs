use super::*;

#[test]
fn scene_navigation_disambiguates_equal_file_names_before_switching() {
    let directory = project(recite_writer_model::FIXTURE).unwrap();
    for folder in ["one", "two"] {
        let parent = directory.path().join(folder);
        fs::create_dir(&parent).unwrap();
        fs::write(
            parent.join("scene.recite"),
            format!(":: {folder}\n-> END\n"),
        )
        .unwrap();
    }
    let mut writer = open(&directory).unwrap();
    let captions = labels(&mut writer).unwrap();
    assert!(
        captions
            .iter()
            .any(|name| name == "Scene · one/scene.recite")
    );
    assert!(
        captions
            .iter()
            .any(|name| name == "Scene · two/scene.recite")
    );
    assert!(!support::is_expanded(
        &mut writer,
        "Scene · two/scene.recite"
    ));
    click(&mut writer, &["Scene · two/scene.recite"]).unwrap();
    assert_eq!(
        writer.session.project().unwrap().current().path(),
        directory.path().join("two/scene.recite")
    );
    // The open scene's disclosure reports its state; its name carries no glyph.
    assert!(support::is_expanded(
        &mut writer,
        "Scene · two/scene.recite"
    ));
}

#[test]
fn inactive_draft_guards_window_closing_and_save_close_covers_every_open_file() {
    let directory = project(recite_writer_model::FIXTURE).unwrap();
    let other = directory.path().join("other.recite");
    fs::write(&other, ":: other\n> @m3ffvrt\n  Other paragraph.\n-> END\n").unwrap();
    let mut writer = open(&directory).unwrap();
    writer.change(|model| {
        model.set_draft("Keep my first draft.".into());
        Ok(())
    });
    click(&mut writer, &["Other"]).unwrap();
    assert_eq!(writer.session.project().unwrap().current().path(), other);
    assert!(!writer.session.project().unwrap().current().dirty());
    click(&mut writer, &["Workspace", "Close"]).unwrap();
    assert!(writer.phase() == Phase::ConfirmClose);
    click(&mut writer, &["Keep editing"]).unwrap();
    assert!(writer.phase() == Phase::Writing);
    click(&mut writer, &["Workspace", "Close", "Save all and close"]).unwrap();
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
    click(&mut writer, &["Other"]).unwrap();
    fs::write(
        directory.path().join("scene.recite"),
        ":: external\n-> END\n",
    )
    .unwrap();
    click(&mut writer, &["Workspace", "Close", "Save all and close"]).unwrap();
    assert!(writer.keep_running());
    assert!(writer.phase() == Phase::ConfirmClose);
    assert!(writer.status.contains("scene.recite"));
    assert!(writer.status.contains("changed on disk"));
    click(&mut writer, &["Keep editing"]).unwrap();
    writer.switch(&directory.path().join("missing.recite"));
    assert_eq!(writer.session.project().unwrap().current().path(), other);
}

#[test]
fn every_file_is_a_row_without_opening_unvisited_documents() {
    let directory = project(recite_writer_model::FIXTURE).unwrap();
    for index in 0..12 {
        fs::write(
            directory.path().join(format!("file_{index:02}.recite")),
            format!(":: file_{index}\n-> END\n"),
        )
        .unwrap();
    }
    let mut writer = open(&directory).unwrap();
    let labels = labels(&mut writer).unwrap();
    assert!(labels.iter().any(|label| label == "File 00"));
    assert!(labels.iter().any(|label| label == "File 11"));
    assert!(
        !labels
            .iter()
            .any(|label| label == "Next files" || label == "Previous files"),
        "the scene list scrolls instead of paging"
    );
    assert_eq!(writer.session.project().unwrap().sessions().count(), 1);
}
