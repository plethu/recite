use super::*;
use crate::localisation::tests::{fixture, key};
use std::fs;

#[test]
fn translation_field_save_publishes_only_its_entry_without_applying_manuscript() {
    let (directory, mut writer, mut driver) = fixture();
    let key = key(&writer);
    let target = TranslationTarget(key.clone());
    writer.change(|model| {
        model.set_draft("Unapplied private manuscript.".into());
        Ok(())
    });
    let before = writer.session.model().recovery();
    let source = fs::read(directory.path().join("scene.recite")).unwrap();
    writer.update_translation(&key, 0, "Translation shortcut saved this.".into());
    writer.save_focused_translation(&target);
    assert!(writer.localisation.saving());
    assert!(!writer.work.busy() && !writer.navigation.busy());
    driver.finish(&mut writer);
    assert!(!writer.localisation.dirty());
    assert_eq!(writer.session.model().recovery(), before);
    assert_eq!(
        fs::read(directory.path().join("scene.recite")).unwrap(),
        source
    );
    assert!(
        fs::read_to_string(directory.path().join("fr.po"))
            .unwrap()
            .contains("Translation shortcut saved this.")
    );
}

#[test]
fn queued_translation_save_rejects_replaced_owner_hidden_entry_and_closing_scope() {
    for change in 0..4 {
        let (directory, mut writer, _driver) = fixture();
        let key = key(&writer);
        let target = TranslationTarget(key.clone());
        writer.update_translation(&key, 0, "Never published by a stale command.".into());
        match change {
            0 => writer
                .localisation
                .slot
                .owner_mut()
                .unwrap()
                .renew_identity(),
            1 => writer.localisation.destination = Destination::Queue,
            2 => writer.workspaces.screen = Screen::Write,
            _ => writer.phase = Phase::ConfirmClose,
        }
        writer.save_focused_translation(&target);
        assert!(!writer.localisation.saving());
        assert!(!writer.project_jobs_busy());
        assert!(
            !fs::read_to_string(directory.path().join("fr.po"))
                .unwrap()
                .contains("Never published")
        );
    }
}
