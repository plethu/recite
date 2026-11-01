use super::*;

#[test]
fn returning_to_writing_hides_preview_and_accepts_unchanged_preparation_in_the_background() {
    let directory = project(SOURCE).unwrap();
    let mut writer = open(&directory).unwrap();
    writer.configure_trial();
    writer.trial.settings.variant = "formal".into();
    writer.return_from_trial();
    assert!(!writer.trial.visible);
    assert_eq!(writer.trial.settings.variant, "formal");
    writer.trial.settings.variant.clear();
    writer.configure_trial();
    let request = begin(&mut writer).unwrap();
    writer.return_from_trial();
    assert!(!writer.trial.visible);
    assert!(!writer.trial.settings.open);
    assert!(finish(&mut writer, request).unwrap().is_none());
    assert!(
        !writer.trial.visible,
        "background completion must not reopen Preview"
    );
    assert_eq!(
        writer.session.model().preview_page().unwrap().text,
        "Hello."
    );
}

#[test]
fn preview_configuration_and_checked_pin_preserve_the_running_position_and_snapshot() {
    let directory = project(SOURCE).unwrap();
    let mut writer = open(&directory).unwrap();
    install(&mut writer).unwrap();
    click(&mut writer, &["Continue preview"]).unwrap();
    assert!(writer.session.model().preview_page().unwrap().ended);
    writer.configure_trial();
    writer.return_from_trial();
    assert!(!writer.trial.visible);
    writer.publish_reference().unwrap();
    assert!(!writer.trial.visible);
    assert!(!writer.trial.settings.open);
    writer.configure_trial();
    assert!(writer.session.model().preview_page().unwrap().ended);
    writer.show_workspace(Workspace::Script);
    assert!(!writer.trial.visible);
    writer.configure_trial();
    assert_eq!(
        writer
            .reading_context
            .pinned
            .as_ref()
            .unwrap()
            .snapshot
            .block()
            .id,
        "start"
    );
    writer.change(|model| {
        model.set_draft("Pending prose.".into());
        Ok(())
    });
    assert!(writer.publish_reference().is_err());
    assert!(
        writer.trial.visible,
        "a rejected pin must not hide playback"
    );
    assert!(writer.session.model().preview_page().unwrap().ended);
}

#[test]
fn hidden_preparation_rejects_intervening_edits_and_explicit_cancellation() {
    let directory = project(SOURCE).unwrap();
    let mut writer = open(&directory).unwrap();
    install(&mut writer).unwrap();
    for cancel in [false, true] {
        let request = begin(&mut writer).unwrap();
        writer.return_from_trial();
        if cancel {
            writer.trial.cancel();
        } else {
            writer.change(|model| {
                model.set_draft("Newer writing.".into());
                Ok(())
            });
        }
        assert!(matches!(
            finish(&mut writer, request).unwrap(),
            Some(Retired::Prepared(_))
        ));
        assert!(!writer.trial.visible);
        assert_eq!(
            writer.session.model().preview_page().unwrap().text,
            "Hello."
        );
        writer.change(|model| {
            model.discard();
            Ok(())
        });
    }
}
