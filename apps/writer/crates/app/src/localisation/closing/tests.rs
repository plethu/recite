use super::*;
use crate::{
    localisation::tests::{SOURCE, fixture, key},
    test_support as support,
};
use std::fs;

#[test]
fn discard_and_close_removes_every_translation_draft_but_retains_source_recovery() {
    let (directory, mut writer, mut driver) = fixture();
    let path = directory.path().join("fr.po");
    writer.catalogue_action(work::Action::Close);
    driver.finish(&mut writer);
    fs::write(&path, "msgctxt \"ffee6p8\"\nmsgid \"Hello.\"\nmsgstr \"Bonjour.\"\n\nmsgctxt \"rthh7ab\"\nmsgid \"Goodbye.\"\nmsgstr \"Au revoir.\"\n").unwrap();
    writer.open_catalogue_path();
    driver.finish(&mut writer);
    let ids = writer
        .localisation
        .slot
        .document()
        .unwrap()
        .entries()
        .iter()
        .map(|entry| entry.id())
        .collect::<Vec<_>>();
    for id in ids {
        let key = EntryKey {
            catalogue: writer.localisation.slot.identity().unwrap().clone(),
            id,
        };
        writer.inspect_translation(&key);
        driver.finish(&mut writer);
        writer.update_translation(&key, 0, "Discard this draft.".into());
    }
    assert_eq!(
        writer
            .localisation
            .slot
            .owner()
            .unwrap()
            .catalogue
            .changed_entries()
            .len(),
        2
    );
    writer.change(|model| {
        model.set_draft("Keep source recovery.".into());
        Ok(())
    });
    writer.request_close();
    writer.discard_translations_and_close();
    assert!(writer.phase == Phase::Closing);
    driver.finish(&mut writer);
    assert!(writer.phase == Phase::Closed, "{}", writer.status);
    assert_eq!(
        fs::read_to_string(directory.path().join("scene.recite")).unwrap(),
        SOURCE
    );
    drop(writer);
    let reopened = Catalogue::open_recoverable(&path).unwrap();
    assert!(!reopened.dirty());
    assert!(!reopened.document.source().contains("Discard this draft"));
    let writer = support::open(&directory).unwrap();
    assert_eq!(writer.session.model().draft(), "Keep source recovery.");
}

#[test]
fn failed_discard_recovery_preflight_keeps_window_and_drafts_open() {
    let (directory, mut writer, mut driver) = fixture();
    let key = key(&writer);
    writer.update_translation(&key, 0, "Keep after failed fence.".into());
    writer
        .localisation
        .slot
        .owner_mut()
        .unwrap()
        .flush_recovery()
        .unwrap();
    let recovery = directory.path().join("fr.po.recite-editor-recovery.json");
    fs::remove_file(&recovery).unwrap();
    fs::create_dir(&recovery).unwrap();
    writer.update_translation(&key, 0, "Newer undurable translation.".into());
    writer.request_close();
    writer.discard_translations_and_close();
    driver.finish(&mut writer);
    assert!(writer.phase == Phase::ConfirmClose);
    assert!(writer.localisation.dirty());
    assert_eq!(
        writer.localisation.slot.draft(key.id).unwrap().forms,
        ["Newer undurable translation."]
    );
    fs::remove_dir(&recovery).unwrap();
    writer.discard_translations_and_close();
    driver.finish(&mut writer);
    assert!(writer.phase == Phase::Closed, "{}", writer.status);
}
